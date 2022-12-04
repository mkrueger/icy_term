use std::{
    cmp::min, sync::{Arc, Mutex}, time::Duration,
};

use crate::{
    protocol::{
        zfile_flag, FileDescriptor, FrameType, Header, HeaderType, TransferState, Zmodem, ZCRCE,
        ZCRCG, zmodem::error::TransmissionError,
    }, com::{Com, ComResult}, 
};

use super::{ZCRCW, ZCRCQ};

#[derive(Debug)]
pub enum SendState {
    Await,
    AwaitZRPos,
    SendZRQInit,
    SendZDATA,
    SendDataPackages,
    Finished,
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
    retries: usize,
    can_count: usize,
    receiver_capabilities: u8,
}

impl Sz {
    pub fn new(block_length: usize) -> Self {
        Self {
            state: SendState::Finished,
            files: Vec::new(),
            cur_file: 0,
            transfered_file: false,
            cur_file_pos: 0,
            errors: 0,
            data: Vec::new(),
            retries: 0,
            receiver_capabilities: 0,
            can_count: 0,
            package_len: block_length,
        }
    }

    fn _can_fdx(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANFDX != 0
    }
    fn _can_receive_data_during_io(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANOVIO != 0
    }
    fn _can_send_break(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANBRK != 0
    }
    fn _can_decrypt(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANCRY != 0
    }
    fn _can_lzw(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANLZW != 0
    }
    fn _can_use_crc32(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::CANFC32 != 0
    }
    fn _can_esc_control(&self) -> bool {
        self.receiver_capabilities | super::zrinit_flag::ESCCTL != 0
    }
    fn _can_esc_8thbit(&self) -> bool {
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
        match self.get_header_type() {
            HeaderType::Bin | HeaderType::Hex => Zmodem::encode_subpacket_crc16(zcrc_byte, data),
            HeaderType::Bin32 => Zmodem::encode_subpacket_crc32(zcrc_byte, data),
        }
    }

    pub fn is_active(&self) -> bool {
        if let SendState::Finished = self.state {
            false
        } else {
            true
        }
    }

    fn next_file(&mut self) {
        self.cur_file += 1;
    }

    pub async fn update(&mut self, com: &mut Box<dyn Com>, transfer_state: Arc<Mutex<TransferState>>) -> ComResult<()> {
        if let SendState::Finished = self.state {
            return Ok(());
        }
        if self.retries > 5 {
            Zmodem::cancel(com).await?;
            self.state = SendState::Finished;
            return Ok(());
        }
 
        if let Ok(transfer_state) = &mut transfer_state.lock() {
            let transfer_info = &mut transfer_state.send_state;
            if self.cur_file >= 0 && self.cur_file < self.files.len() as i32 {
                let fd = &self.files[self.cur_file as usize];
                transfer_info.file_name = fd.file_name.clone();
                transfer_info.file_size = fd.size;
            }
            transfer_info.bytes_transfered = self.cur_file_pos;
            transfer_info.errors = self.errors;
            transfer_info.check_size = format!("Crc32/{}", self.package_len);
            transfer_info.update_bps();
        }
 
        match self.state {
            SendState::Await | 
            SendState::AwaitZRPos => {
                self.read_next_header(com).await?;
            }
            SendState::SendZRQInit => {
//                transfer_state.lock().unwrap().current_state = "Negotiating transfer";
            //    let now = SystemTime::now();
           //     if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zrqinit(com).await?;
                    self.read_next_header(com).await?;

                    self.retries += 1;
           //         self.last_send = SystemTime::now();
           //     }
            }
            SendState::SendZDATA => {
  //              transfer_state.lock().unwrap().current_state = "Sending data";
                if self.cur_file < 0 {
                    //println!("no file to send!");
                    return Ok(());
                }
                Header::from_number(
                    self.get_header_type(),
                    FrameType::ZDATA,
                    self.cur_file_pos as u32,
                )
                .write(com).await?;
                self.state = SendState::SendDataPackages;
            }
            SendState::SendDataPackages => {
                let mut p = Vec::new();
                if self.cur_file < 0 {
                    return Ok(());
                }
                let old_pos = self.cur_file_pos;
                let end_pos = min(self.data.len(), self.cur_file_pos + self.package_len);
                let nonstop = true; // self.package_len > 1024;

                let crc_byte = if self.cur_file_pos + self.package_len < self.data.len() {
                    if nonstop { ZCRCG } else { ZCRCQ }
                } else {
                    ZCRCE
                };
                p.extend_from_slice(
                    &self.encode_subpacket(crc_byte, &self.data[self.cur_file_pos..end_pos]),
                );
                self.cur_file_pos = end_pos;
                if end_pos >= self.data.len() {
                    p.extend_from_slice(
                        &Header::from_number(
                            self.get_header_type(),
                            FrameType::ZEOF,
                            end_pos as u32,
                        )
                        .build(),
                    );
                    //transfer_info.write("Done sending file date.".to_string());
                   // transfer_state.lock().unwrap().current_state = "Done data";
                    self.transfered_file = true;
                    self.state = SendState::Await;
                }
                com.send(&p).await?;
                if !nonstop {
                    let ack = Header::read(com, &mut self.can_count).await?;
                    if let Some(header) = ack {
                        match header.frame_type {
                            FrameType::ZACK => { /* ok */},
                            FrameType::ZNAK => { self.cur_file_pos = old_pos;  /* resend */} ,
                            FrameType::ZRPOS => { self.cur_file_pos = header.number() as usize; },
                            _ => {
                                eprintln!("unexpected header {:?}", header);
                                // cancel
                                self.state = SendState::Finished;
                                Zmodem::cancel(com).await?;
                            }
                        }
                    }
                }
                // for some reason for some BBSes it's too fast - adding a little delay here fixes that
                // Note that using ZCRCQ doesn't seem to fix that issue.
                std::thread::sleep(Duration::from_millis(5));
            }
            SendState::Finished => {
//                transfer_state.lock().unwrap().current_state = "Finishing transfer…";
                // let now = SystemTime::now();
                //if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zfin(com, 0).await?;
                //}
                return Ok(());
            }
        }
        Ok(())
    }

    async fn read_next_header(&mut self, com: &mut Box<dyn Com>) -> ComResult<()>  
    {
        let err = Header::read(com, &mut self.can_count).await;
        if self.can_count >= 5 {
            // transfer_info.write("Received cancel...".to_string());
            self.state = SendState::Finished;
            return Ok(());
        }
        if let Err(err) = err {
            println!("{}", err);
            if self.errors > 3 {
                self.state = SendState::Finished;
                Zmodem::cancel(com).await?;
                return Err(err);
            }
            self.errors += 1;
            return Ok(());
        }
        self.errors = 0;
        let res = err.unwrap();
        if let Some(res) = res {
            println!("Recv header {} {:?}", res, self.state);
            match res.frame_type {
                FrameType::ZRINIT => {
                    if self.transfered_file {
                        self.next_file();
                        self.transfered_file = false;
                    }

                    if self.cur_file as usize >= self.files.len() {
                        self.state = SendState::Await;
                        self.send_zfin(com, self.cur_file_pos as u32).await?;
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
                  //  transfer_state.lock().unwrap().current_state = "Sending header";
                    self.send_zfile(com).await?;
                    self.state = SendState::AwaitZRPos;
                    return Ok(());
                }
        
        
                FrameType::ZNAK => {
                    // transfer_info
                    //     .write("Package error, resending file header...".to_string());
                }

                FrameType::ZRPOS => {
                    self.cur_file_pos = res.number() as usize;
                    self.state = SendState::SendZDATA;

                    if let SendState::SendDataPackages = self.state {
                        if self.package_len > 512 {
                            //reinit transfer.
                            self.package_len /= 2;
                            self.state = SendState::SendZRQInit;
                            //        com.write(b"rz\r")?;
                            self.send_zrqinit(com).await?;
                            return Ok(());
                        }
                    }
                }

                FrameType::ZFIN => {
                    self.state = SendState::Finished;
                    com.send(b"OO").await?;
                    return Ok(());
                }

                FrameType::ZSKIP => {
                   // transfer_state.lock().unwrap().current_state = "Skipped… next file";
                    //transfer_info.write("Skip file".to_string());
                    self.next_file();
                    self.send_zfile(com).await?;
                    return Ok(());
                }

                FrameType::ZACK => {
                    self.state = SendState::SendDataPackages;
                }
                FrameType::ZCHALLENGE => {
                    Header::from_number(
                        self.get_header_type(),
                        FrameType::ZACK,
                        res.number(),
                    )
                    .write(com).await?;
                }
                FrameType::ZABORT | FrameType::ZFERR | FrameType::ZCAN => {
                    Header::empty(self.get_header_type(), FrameType::ZFIN).write(com).await?;
                    self.state = SendState::Finished;
                }
                unk_frame => {
                    return Err(Box::new(TransmissionError::UnsupportedFrame(unk_frame)));
                }
            }
        }
        Ok(())
    }

    async fn send_zfile(
        &mut self,
        com: &mut Box<dyn Com>
    ) -> ComResult<()> {
        if self.cur_file < 0 {
            return Ok(());
        }
        let mut b = Vec::new();
        //transfer_state.write("Send file header".to_string());
        b.extend_from_slice(
            &Header::from_flags(
                self.get_header_type(),
                FrameType::ZFILE,
                0,
                0,
                zfile_flag::ZMNEW,
                zfile_flag::ZCRESUM,
            )
            .build(),
        );

        let f = &self.files[self.cur_file as usize];
        self.data = f.get_data()?;
        let data = if f.date > 0 {
            let bytes_left = self
                .files
                .iter()
                .skip(self.cur_file as usize + 1)
                .fold(0, |b, f| b + f.size);
            format!(
                "{}\0{} {} 0 0 {} {}\0",
                f.file_name,
                f.size,
                f.date,
                self.files.len() - self.cur_file as usize,
                bytes_left
            )
            .into_bytes()
        } else {
            format!("{}\0{}\0", f.file_name, f.size).into_bytes()
        };

        b.extend_from_slice(&self.encode_subpacket(ZCRCW, &data));

        com.send(&b).await?;


        self.cur_file_pos = 0;
        self.state = SendState::AwaitZRPos;
        Ok(())
    }

    pub async fn send(&mut self, _com: &mut Box<dyn Com>, files: Vec<FileDescriptor>) -> ComResult<()> {
        //println!("initiate zmodem send {}", files.len());
        self.state = SendState::SendZRQInit;
        self.files = files;
        self.cur_file = 0;
        self.cur_file_pos = 0;
        self.retries = 0;
//        com.write(b"rz\r")?;
        Ok(())
    }

    pub async fn send_zrqinit(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        self.cur_file = -1;
        self.transfered_file = true;
        Header::empty(self.get_header_type(), FrameType::ZRQINIT).write(com).await?;
        Ok(())
    }

    pub async fn send_zfin(&mut self, com: &mut Box<dyn Com>, size: u32) -> ComResult<()> {
        println!("send zfin!");
        Header::from_number(self.get_header_type(), FrameType::ZFIN, size).write(com).await?;
        self.state = SendState::Await;
        Ok(())
    }
}
