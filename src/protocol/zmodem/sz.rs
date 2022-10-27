use std::{io, cmp::min, time::{SystemTime, Duration}};

use crate::{com::Com, protocol::{FileDescriptor, Zmodem, frame_types, zrinit_flag, zfile_flag, ZCRCW, ZCRCG, HeaderType, Header}};

#[derive(Debug)]
pub enum SendState {
    Idle,
    Await,
    SendZRQInit,
    SendZDATA,
    SendDataPackages,
    SendZFILE
}

pub struct Sz {
    state: SendState,
    pub files: Vec<FileDescriptor>,
    cur_file: usize,
    cur_file_pos: usize,
    pub bytes_send: usize,
    pub errors: usize,
    pub package_len: usize,
    data: Vec<u8>,
    last_send: SystemTime,
    retries: usize,

    receiver_capabilities: u8
}

impl Sz {
    pub fn new() -> Self
    {
        Self {
            state: SendState::Idle,
            files: Vec::new(),
            cur_file: 0,
            cur_file_pos: 0,
            bytes_send: 0,
            errors: 0,
            data: Vec::new(),
            last_send: SystemTime::now(),
            retries: 0,
            receiver_capabilities: 0,
            package_len: 2048
        }
    }

    fn can_fdx(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::CANFDX != 0
    }
    fn can_receive_data_during_io(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::CANOVIO != 0
    }
    fn can_send_break(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::CANBRK != 0
    }
    fn can_decrypt(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::CANCRY != 0
    }
    fn can_lzw(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::CANLZW != 0
    }
    fn can_use_crc32(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::CANFC32 != 0
    }
    fn can_esc_control(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::ESCCTL != 0
    }
    fn can_esc_8thbit(&self) -> bool {
        self.receiver_capabilities | zrinit_flag::ESC8 != 0
    }

    pub fn is_active(&self) -> bool
    {
        if let SendState::Idle = self.state {
            false
        } else {
            true 
        }
    }

    pub fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        if let SendState::Idle = self.state {
            return Ok(());
        }
        if self.retries > 5  {
            Zmodem::cancel(com)?;
            self.state = SendState::Idle;
            return Ok(());
        }
        let err = Header::read(com);
        if err.is_err() {
            println!("Last packet had error sending ZNAK");
            self.errors += 1;
            com.write(&Header::empty(HeaderType::Bin32, frame_types::ZNAK).build())?;
            return Ok(());
        }
        let res = err?;
        if let Some(res) = res {
            println!("got frame {:02X}", res.frame_type);
            match res.frame_type {
                frame_types::ZRINIT => {
                    if self.cur_file >= self.files.len() {
                        self.state = SendState::Idle;
                        com.write(&Header::empty(HeaderType::Bin32,frame_types::ZFIN).build())?;
                        return Ok(());
                    }
                    self.receiver_capabilities = res.f0();
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
                    }
                    self.state = SendState::SendZFILE;
                }
                frame_types::ZNAK => {
                    println!("ZNAK, restarting current file ZFILE");
                    self.state = SendState::SendZFILE;
                }
                frame_types::ZRPOS => {
                    self.cur_file_pos = res.number() as usize;
                    println!("file pos requested {}", self.cur_file_pos);
                    self.state = SendState::SendDataPackages;
                }
                frame_types::ZFIN => {
                    self.state = SendState::Idle;
                    return Ok(());
                }
                unk_frame => {
                    println!("unknown frame {:X}.", unk_frame);
                }
            }
        }

        match self.state {
            SendState::SendZRQInit => {
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    Header::empty(HeaderType::Hex,frame_types::ZRQINIT).write(com)?;
                    self.retries += 1;
                    self.last_send = SystemTime::now();

                }
            }
            SendState::SendZFILE => {
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    println!("Send ZFILE");
                    let mut b = Vec::new();
                    b.extend_from_slice(&Header::from_flags(HeaderType::Bin32,frame_types::ZRQINIT, 0, 0, zfile_flag::ZMNEW, zfile_flag::ZCRESUM).build());
                    

                    let f = &self.files[self.cur_file];
                    self.data = f.get_data()?;
                    let data = if f.date > 0 { 
                        format!("{}\0{}\0{}\0", f.file_name, f.size, f.date).into_bytes()
                    }  else {
                        format!("{}\0{}\0", f.file_name, f.size).into_bytes()
                    };
                    b.extend_from_slice(&Zmodem::encode_subpacket_crc32(ZCRCW, &data));
                    com.write(&b)?;

                    self.retries += 1;
                    self.last_send = SystemTime::now();
                    self.state = SendState::SendZDATA;
                }
            }
            SendState::SendZDATA => {
                if self.cur_file >= self.files.len() {
                    self.state = SendState::Idle;
                    Header::empty(HeaderType::Bin32,frame_types::ZFIN).write(com)?;
                    return Ok(());
                }
                if self.bytes_send >= self.files[self.cur_file].size {
                    Header::from_number(HeaderType::Bin32,frame_types::ZEOF, self.files[self.cur_file].size as u32).write(com)?;
                    self.cur_file += 1;
                    self.state = SendState::Await;
                    return Ok(());
                }
                Header::from_number(HeaderType::Bin32,frame_types::ZDATA, self.cur_file_pos as u32).write(com)?;
                self.state = SendState::SendDataPackages;
            }
            SendState::SendDataPackages => {
                if self.cur_file >= self.files.len() {
                    self.state = SendState::Idle;
                    Header::empty(HeaderType::Bin32,frame_types::ZFIN).write(com)?;
                    return Ok(());
                }
                if self.cur_file_pos >= self.files[self.cur_file].size {
                    println!("END!");
                    Header::from_number(HeaderType::Bin32,frame_types::ZEOF, self.files[self.cur_file].size as u32).write(com)?;
                    self.cur_file += 1;
                    self.state = SendState::Await;
                    return Ok(());
                }

                let end_pos = min(self.data.len(), self.cur_file_pos + self.package_len);
                com.write(&Zmodem::encode_subpacket_crc32(ZCRCG, &self.data[self.cur_file_pos..end_pos]))?;
                self.cur_file_pos = end_pos;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn send<T: Com>(&mut self, _com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        println!("initiate zmodem send {}", files.len());
        self.state = SendState::SendZRQInit;
        self.files = files;
        self.cur_file = 0;
        self.bytes_send = 0;
        self.last_send = SystemTime::now().checked_sub(Duration::from_secs(100)).unwrap();
        self.retries = 0;
        Ok(())
    }

}

