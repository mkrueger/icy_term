use std::{io::{self, ErrorKind}, cmp::min, time::{SystemTime}};

use crate::{com::Com, protocol::{FileDescriptor, Zmodem, FrameType, zfile_flag, ZCRCG, HeaderType, Header, ZCRCE, TransferState}};

use super::ZCRCW;

#[derive(Debug)]
pub enum SendState {
    Idle,
    Await,
    AwaitZRPos,
    SendZRQInit,
    SendZDATA,
    SendDataPackages,
    Finished
}

pub struct Sz {
    state: SendState,
    pub files: Vec<FileDescriptor>,
    cur_file: i32,
    cur_file_pos: usize,
    pub errors: usize,
    pub package_len: usize,
    pub transfered_file: bool,
    data: Vec<u8>,
    last_send: SystemTime,
    retries: usize,
    can_count: usize,
    receiver_capabilities: u8
}

impl Sz {
    pub fn new(block_length: usize) -> Self
    {
        Self {
            state: SendState::Idle,
            files: Vec::new(),
            cur_file: 0,
            transfered_file: false,
            cur_file_pos: 0,
            errors: 0,
            data: Vec::new(),
            last_send: SystemTime::now(),
            retries: 0,
            receiver_capabilities: 0,
            can_count: 0,
            package_len: block_length
        }
    }
 
    fn can_fdx(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANFDX != 0
    }
    fn can_receive_data_during_io(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANOVIO != 0
    }
    fn can_send_break(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANBRK != 0
    }
    fn can_decrypt(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANCRY != 0
    }
    fn can_lzw(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANLZW != 0
    }
    fn can_use_crc32(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANFC32 != 0
    }
    fn can_esc_control(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::ESCCTL != 0
    }
    fn can_esc_8thbit(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::ESC8 != 0
    }

    fn get_header_type(&self) -> HeaderType {
        // Other headers fall back to crc16 
        // And the original crc16 implementation has a bug which isn't shared with only a few implementations these days crc32 is safe.
        HeaderType::Bin32
/* 
        if self.can_esc_control() || self.can_esc_8thbit() {
            HeaderType::Hex
        } else {
            if self.can_use_crc32() {
                HeaderType::Bin32
            }  else {
                HeaderType::Bin
            }
        }*/
    }

    fn encode_subpacket(&self, zcrc_byte: u8, data: &[u8]) -> Vec<u8> {
        match self.get_header_type()  {
            HeaderType::Bin |
            HeaderType::Hex => Zmodem::encode_subpacket_crc16(zcrc_byte, data),
            HeaderType::Bin32 => Zmodem::encode_subpacket_crc32(zcrc_byte, data)
        }
    }

    pub fn is_active(&self) -> bool
    {
        if let SendState::Idle = self.state {
            false
        } else {
            true 
        }
    }
    
    fn next_file(&mut self)
    {
        self.cur_file += 1;
    }

    pub fn update(&mut self, com: &mut Box<dyn Com>, state: &mut TransferState) -> io::Result<()>
    {
        if let SendState::Idle = self.state {
            return Ok(());
        }
        if self.retries > 5  {
            Zmodem::cancel(com)?;
            self.state = SendState::Idle;
            return Ok(());
        }

        if let Some(transfer_state) = &mut state.send_state {
            if self.cur_file >= 0 && self.cur_file < self.files.len() as i32 {
                let fd = &self.files[self.cur_file as usize];
                transfer_state.file_name = fd.file_name.clone();
                transfer_state.file_size = fd.size;
            }
            transfer_state.bytes_transfered = self.cur_file_pos;
            transfer_state.errors = self.errors;
            transfer_state.check_size = format!("Crc32/{}", self.package_len);
            transfer_state.update_bps();

            if com.is_data_available()? {
                let err = Header::read(com, &mut self.can_count);
                if let Err(err) = err {
                    println!("{}", err);
                    if self.errors > 3 {
                        self.state = SendState::Idle;
                        Zmodem::cancel(com)?;
                        return Err(err);
                    }
                    self.errors += 1;
                    return Ok(());
                }
                self.errors = 0;

                let res = err?;
                if let Some(res) = res {
                    //println!("Recv header {} {:?}", res, self.state);
                    self.last_send = SystemTime::UNIX_EPOCH;
                    match res.frame_type {
                        FrameType::ZRINIT => {
                            if self.transfered_file {
                                self.next_file();
                                self.transfered_file  = false;
                            }

                            if self.cur_file as usize >= self.files.len() {

                                self.state = SendState::Finished;
                                self.send_zfin(com, self.cur_file_pos as u32)?;
                                self.cur_file_pos = 0;
                                return Ok(());
                            }
                            self.cur_file_pos = 0;
                            self.receiver_capabilities = res.f0();

                            /* 
                            if self.can_decrypt() {
                                println!("receiver can decrypt");
                            }
                            if self.can_fdx() {
                                println!("receiver can send and receive true full duplex");
                            }
                            if self.can_receive_data_during_io() {
                                println!("receiver can receive data during disk I/O");
                            }
                            if self.can_send_break() {
                                println!("receiver can send a break signal");
                            }
                            if self.can_lzw() {
                                println!("receiver can uncompress");
                            }
                            if self.can_use_crc32() {
                                println!("receiver can use 32 bit Frame Check");
                            }
                            if self.can_esc_control() {
                                println!("receiver expects ctl chars to be escaped");
                            }
                            if self.can_esc_8thbit() {
                                println!("receiver expects 8th bit to be escaped");
                            }*/
                            state.current_state = "Sending header";
                            self.send_zfile(com, transfer_state)?;
                            self.state = SendState::AwaitZRPos;
                            return Ok(());
                        }
                        FrameType::ZNAK => {
                            transfer_state.write("Package error, resending file header...".to_string());
                        }
                        FrameType::ZRPOS => {
                            self.cur_file_pos = res.number() as usize;
                            self.last_send = SystemTime::UNIX_EPOCH;
                            self.state = SendState::SendZDATA;

                            if let SendState::SendDataPackages = self.state {
                                if self.package_len > 512 {
                                    //reinit transfer.
                                    self.package_len /= 2;
                                    self.state = SendState::SendZRQInit;
                                    self.last_send = SystemTime::now();
                            //        com.write(b"rz\r")?;
                                    self.send_zrqinit(com)?;
                                    return Ok(());
                                }
                            }
                        }
                        FrameType::ZFIN => {
                            self.state = SendState::Idle;
                            com.write(b"OO")?;
                            return Ok(());
                        }
                        FrameType::ZSKIP => {
                            transfer_state.write("Skip file".to_string());
                            self.next_file();
                            state.current_state = "Skipped… next file";
                            self.send_zfile(com, transfer_state)?;
                            return Ok(());
                        }
                        FrameType::ZACK => {
                            self.state = SendState::SendDataPackages;
                        }
                        FrameType::ZCHALLENGE => {
                            Header::from_number(self.get_header_type(), FrameType::ZACK, res.number()).write(com)?;
                        }
                        FrameType::ZABORT | FrameType::ZFERR | FrameType::ZCAN => {
                            Header::empty(self.get_header_type(), FrameType::ZFIN).write(com)?;
                            self.state = SendState::Idle;
                        }
                        unk_frame => {
                            return Err(io::Error::new(ErrorKind::InvalidInput, format!("unsupported frame {:?}.", unk_frame))); 
                        }
                    }
                }
            }
            if let SendState::SendZDATA = self.state { 
            } else 
            if self.cur_file >= 0 {
                if self.cur_file >= self.files.len() as i32 {
                    let now = SystemTime::now();
                    if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                        self.send_zfin(com, 0)?;
                        self.last_send = SystemTime::now();

                    }
                    self.state = SendState::Await;
                    return Ok(());
                }

                if self.cur_file_pos >= self.files[self.cur_file as usize].size {
                    let now = SystemTime::now();
                    if now.duration_since(self.last_send).unwrap().as_millis() > 6000 {
                        Header::from_number(self.get_header_type(), FrameType::ZEOF, self.files[self.cur_file as usize].size as u32).write(com)?;
                        self.state = SendState::Await;
                        self.last_send = SystemTime::now();
                    }
                    return Ok(());
                }
            }
            // println!("State: {:?} cur file {} pos {}", self.state, self.cur_file, self.cur_file_pos);
            match self.state {
            SendState::SendZRQInit => {
                state.current_state = "Negotiating transfer";
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zrqinit(com)?;
                    self.retries += 1;
                    self.last_send = SystemTime::now();
                }
            }
            SendState::SendZDATA => {
                state.current_state = "Sending data";
                if self.cur_file < 0 {
                    //println!("no file to send!");
                    return Ok(());
                }
                Header::from_number(self.get_header_type(),FrameType::ZDATA, self.cur_file_pos as u32).write(com)?;
                self.state = SendState::SendDataPackages;
            }
            SendState::SendDataPackages => {
                let mut p = Vec::new();
                if self.cur_file < 0 {
                    return Ok(());
                }
                let end_pos = min(self.data.len(), self.cur_file_pos + self.package_len);
                let crc_byte = if self.cur_file_pos + self.package_len < self.data.len() { ZCRCG } else { ZCRCE };
                p.extend_from_slice(&self.encode_subpacket(crc_byte, &self.data[self.cur_file_pos..end_pos]));
                self.cur_file_pos = end_pos;

                if end_pos >= self.data.len() {
                    p.extend_from_slice(&Header::from_number(self.get_header_type(), FrameType::ZEOF, end_pos as u32).build());
                    transfer_state.write("Done sending file date.".to_string());
                    state.current_state = "Done data";
                    self.transfered_file = true;
                    self.state = SendState::Finished;
                    self.last_send = SystemTime::now();
                }
                com.write(&p)?;
            }
            SendState::Finished => {
                state.current_state = "Finishing transfer…";
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zfin(com, 0)?;

                }
                return Ok(());
            }
            _ => {}
        }
        
        }
        Ok(())
    }

    fn send_zfile(&mut self, com: &mut Box<dyn Com>, transfer_state: &mut crate::protocol::FileTransferState) -> Result<(), io::Error> {
        if self.cur_file < 0 {
            return Ok(());
        }
        let mut b = Vec::new();
        transfer_state.write("Send file header".to_string());
        b.extend_from_slice(&Header::from_flags(self.get_header_type(), FrameType::ZFILE, 0, 0, zfile_flag::ZMNEW, zfile_flag::ZCRESUM).build());

        let f = &self.files[self.cur_file as usize];
        self.data = f.get_data()?;
        let data = if f.date > 0 { 
            let bytes_left = self.files.iter().skip(self.cur_file as usize + 1).fold(0, |b, f| b + f.size);
            format!("{}\0{} {} 0 0 {} {}\0", f.file_name, f.size, f.date, self.files.len() - self.cur_file as usize, bytes_left).into_bytes()
        }  else {
            format!("{}\0{}\0", f.file_name, f.size).into_bytes()
        };

        b.extend_from_slice(&self.encode_subpacket(ZCRCW, &data));

        com.write(&b)?;
        self.cur_file_pos = 0;
        self.last_send = SystemTime::now();
        self.state = SendState::AwaitZRPos;
        Ok(())
    }

    pub fn send(&mut self, com: &mut Box<dyn Com>, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        //println!("initiate zmodem send {}", files.len());
        self.state = SendState::SendZRQInit;
        self.files = files;
        self.cur_file = 0;
        self.cur_file_pos = 0;
        self.last_send = SystemTime::now();
        self.retries = 0;
//        com.write(b"rz\r")?;
        self.send_zrqinit(com)?;
        Ok(())
    }

    pub fn send_zrqinit(&mut self, com: &mut Box<dyn Com>) -> io::Result<()> {
        self.cur_file = -1;
        self.transfered_file = true;
        Header::empty(self.get_header_type(), FrameType::ZRQINIT).write(com)?;
        Ok(())
    }

    pub fn send_zfin(&mut self, com: &mut Box<dyn Com>, size: u32) -> io::Result<()> {
        Header::from_number(self.get_header_type(), FrameType::ZFIN, size).write(com)?;
        self.state = SendState::Finished;
        self.last_send = SystemTime::now();
        Ok(())
    }
    
}

