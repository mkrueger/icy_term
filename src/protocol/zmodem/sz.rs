use std::{io, cmp::min, time::{SystemTime}};

use crate::{com::Com, protocol::{FileDescriptor, Zmodem, FrameType, zrinit_flag, zfile_flag, ZCRCW, ZCRCG, HeaderType, Header, ZCRCE}};

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
    cur_file: i32,
    cur_file_pos: usize,
    pub bytes_send: usize,
    pub errors: usize,
    pub package_len: usize,
    data: Vec<u8>,
    last_send: SystemTime,
    retries: usize,
    can_count: usize,
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
            can_count: 0,
            package_len: 512
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
    
    fn next_file(&mut self)
    {
        self.cur_file += 1;
        self.cur_file_pos = 0;
        println!(" next file {}" ,self.cur_file);
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
        let err = Header::read(com, &mut self.can_count);
        if err.is_err() {
            println!("Last packet had error sending ZNAK: {:?} {}", err.err(), self.can_count);
            if self.can_count > 3 {
                self.state = SendState::Idle;
                return Ok(());
            }
            self.errors += 1;
            Header::empty(HeaderType::Bin32, FrameType::ZNAK).write(com)?;
            return Ok(());
        }
        let res = err?;
        if let Some(res) = res {
            println!("Recv header {}", res);
            self.last_send = SystemTime::UNIX_EPOCH;
            match res.frame_type {
                FrameType::ZRINIT => {
                    self.next_file();

                    if self.cur_file as usize >= self.files.len() {
                        self.state = SendState::Await;
                        self.send_zfin(com)?;
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
                FrameType::ZNAK => {
                    self.state = SendState::SendZFILE;
                }
                FrameType::ZRPOS => {
                    self.cur_file_pos = res.number() as usize;
                    self.state = SendState::SendZDATA;
                }
                FrameType::ZFIN => {
                    self.state = SendState::Idle;
                    return Ok(());
                }
                FrameType::ZSKIP => {
                    self.next_file();
                    self.state = SendState::SendZFILE;
                }
                unk_frame => {
                    println!("unsupported frame {:?}.", unk_frame);
                }
            }
        }

        if self.cur_file >= 0 {
            if self.cur_file >= self.files.len() as i32 {
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zfin(com)?;
                    self.last_send = SystemTime::now();

                }
                self.state = SendState::Await;
                return Ok(());
            }

            if self.cur_file_pos >= self.files[self.cur_file as usize].size {
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    Header::from_number(HeaderType::Bin32,FrameType::ZEOF, self.files[self.cur_file as usize].size as u32).write(com)?;
                    self.state = SendState::Await;
                    self.last_send = SystemTime::now();
                }
                return Ok(());
            }
        }


        match self.state {
            SendState::SendZRQInit => {
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zrqinit(com)?;
                    self.retries += 1;
                    self.last_send = SystemTime::now();
                }
            }
            SendState::SendZFILE => {
                if self.cur_file < 0 {
                    return Ok(());
                }
                let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    let mut b = Vec::new();
                    b.extend_from_slice(&Header::from_flags(HeaderType::Bin32,FrameType::ZFILE, 0, 0, zfile_flag::ZMNEW, zfile_flag::ZCRESUM).build());

                    let f = &self.files[self.cur_file as usize];
                    self.data = f.get_data()?;
                    let data = if f.date > 0 { 
                        format!("{}\0{} {}\0", f.file_name, f.size, f.date).into_bytes()
                    }  else {
                        format!("{}\0{}\0", f.file_name, f.size).into_bytes()
                    };
                    b.extend_from_slice(&Zmodem::encode_subpacket_crc32(ZCRCW, &data));

                    print!("Send ZFILE: ");
                    for x in &b {
                        print!("{:02x}, ", *x);
                    }
                    println!();
            

                    com.write(&b)?;
                    self.cur_file_pos = 0;
                    Header::from_number(HeaderType::Bin32,FrameType::ZDATA, self.cur_file_pos as u32).write(com)?;

                    self.retries += 1;
                    self.last_send = SystemTime::now();
                    self.state = SendState::SendDataPackages;
                }
            }
            SendState::SendZDATA => {
                if self.cur_file < 0 {
                    return Ok(());
                }
                println!("Send ZDATA from {}", self. cur_file_pos);
                Header::from_number(HeaderType::Bin32,FrameType::ZDATA, self.cur_file_pos as u32).write(com)?;
                self.state = SendState::SendDataPackages;
            }
            SendState::SendDataPackages => {
                if self.cur_file < 0 {
                    return Ok(());
                }
                let end_pos = min(self.data.len(), self.cur_file_pos + self.package_len);
                let crc_byte = if end_pos < self.data.len() { ZCRCG } else { ZCRCE };
                println!("Send content data {}: {} bytes crc bytes: {}", self.cur_file_pos, end_pos - self.cur_file_pos, crc_byte);
                let mut p = Zmodem::encode_subpacket_crc32(crc_byte, &self.data[self.cur_file_pos..end_pos]);
     

                for x in &p {
                    print!("{:02x}, ", *x);
                }
                println!();

                com.write(&p)?;


                if end_pos >= self.data.len() {
                    Header::from_number(HeaderType::Bin32,FrameType::ZEOF, self.files[self.cur_file as usize].size as u32).write(com)?;
                    self.last_send = SystemTime::now();
                }

                self.cur_file_pos = end_pos;
            }
            _ => {}
        }
        Ok(())
    }

    pub fn send<T: Com>(&mut self, com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        println!("initiate zmodem send {}", files.len());
        self.state = SendState::SendZRQInit;
        self.files = files;
        self.bytes_send = 0;
        self.last_send = SystemTime::now();
        self.retries = 0;
        com.write(b"rz\r")?;
        self.send_zrqinit(com)?;
        Ok(())
    }

    pub fn send_zrqinit<T: Com>(&mut self, com: &mut T) -> io::Result<()> {
        self.cur_file = -1;
        Header::empty(HeaderType::Hex,FrameType::ZRQINIT).write(com)?;
        Ok(())
    }

    pub fn send_zfin<T: Com>(&mut self, com: &mut T) -> io::Result<()> {
        Header::from_number(HeaderType::Hex,FrameType::ZFIN, self.cur_file as u32).write(com)?;
        Ok(())
    }

}

