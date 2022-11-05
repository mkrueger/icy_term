use std::{io::{self, ErrorKind}, time::{SystemTime, Duration}};

use icy_engine::{get_crc32, update_crc32};

use crate::{com::Com, protocol::{FileDescriptor, Zmodem, FrameType, ZCRCW, ZCRCG, HeaderType, Header, ZCRCE, TransferState, FileTransferState}};

use super::{constants::*, read_zdle_bytes};

#[derive(Debug)]
pub enum RevcState {
    Idle,
    Await,
    AwaitZDATA,
    AwaitFileData,
    AwaitEOF,
    SendZRINIT
}

pub struct Rz {
    state: RevcState,
    pub files: Vec<FileDescriptor>,
    pub errors: usize,
    last_send: SystemTime,
    retries: usize,
    can_count: usize,
    block_length: usize
}

impl Rz {
    pub fn new(block_length: usize) -> Self
    {
        Self {
            state: RevcState::Idle,
            files: Vec::new(),
            last_send: SystemTime::now(),
            block_length,
            retries: 0,
            can_count: 0,
            errors: 0
        }
    }

    pub fn is_active(&self) -> bool
    {
        if let RevcState::Idle = self.state {
            false
        } else {
            true 
        }
    }

    fn cancel(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>
    {
        self.state = RevcState::Idle;
        Zmodem::cancel(com)
    }

    pub fn update(&mut self, com: &mut Box<dyn Com>, state: &mut TransferState) -> io::Result<()>
    {
        if let RevcState::Idle = self.state {
            return Ok(());
        }
        if let Some(transfer_state) = &mut state.recieve_state {
            if self.retries > 5  {
                transfer_state.write("Too many reties cancel...".to_string());
                self.cancel(com)?;
                return Ok(());
            }
    
            if self.files.len() > 0 {
                let cur_file = self.files.len() - 1;
                let fd = &self.files[cur_file];
                transfer_state.file_name = fd.file_name.clone();
                transfer_state.file_size = fd.size;
                transfer_state.bytes_transfered = fd.data.as_ref().unwrap().len();
            }
            transfer_state.errors = self.errors;
            transfer_state.check_size = format!("Crc32");
            transfer_state.update_bps();
        
            // println!("\t\t\t\t\t\tReceiver state {:?}", self.state);
            
            match self.state {
                RevcState::SendZRINIT => {
                    if self.read_header(com, transfer_state)? {
                        return Ok(())
                    }
                    let now = SystemTime::now();
                    if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                        self.send_zrinit(com)?;
                        self.retries += 1;
                        self.last_send = SystemTime::now();
                    }
                }
                RevcState::AwaitZDATA => {
                    let now = SystemTime::now();
                    if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                        self.request_zpos(com)?;
                        self.retries += 1;
                        self.last_send = SystemTime::now();
                    }
                    self.read_header(com, transfer_state)?;
                }
                RevcState::AwaitFileData => {
                    let pck = self.read_subpacket(com)?;
                    let last = self.files.len() - 1;
                    match pck {
                        Some((block, is_last)) => {
                            if let Some(fd) = self.files.get_mut(last) {
                                if let Some(data) = &mut fd.data {
                                    data.extend_from_slice(&block);
                                }
                            }
                            if is_last {
                                self.state = RevcState::AwaitEOF;
                            }
                        }
                        None => {
                            if let Some(fd) = self.files.get(last) {
                                Header::from_number(HeaderType::Hex, FrameType::ZRPOS, fd.data.as_ref().unwrap().len() as u32).write(com)?;
                                self.state = RevcState::AwaitZDATA;
                            }
                            return Ok(());
                        }
                    }
                }
                _ => { self.read_header(com, transfer_state)?; }
            }
        }
        Ok(())
    }

    fn request_zpos(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>
    {
        Header::from_number(HeaderType::Hex, FrameType::ZRPOS, 0).write(com)
    }

    fn read_header(&mut self, com: &mut Box<dyn Com>, transfer_state: &mut FileTransferState) -> io::Result<bool>
    {
        while com.is_data_available()? {
            let result = Header::read(com, &mut self.can_count);
            if let Err(err) = result {
                if self.can_count >= 5 {
                    transfer_state.write("Received cancel...".to_string());
                    self.cancel(com)?;
                    self.cancel(com)?;
                    self.cancel(com)?;
                    self.state = RevcState::Idle;
                    return Ok(false);
                }
                println!("{}", err);
                transfer_state.write(format!("{}", err));
                self.errors += 1;
                continue;
            }
            self.can_count = 0;
            let res = result?;
            if let Some(res) = res {
                println!("\t\t\t\t\t\tRECV header {}", res);
                match res.frame_type {
                    FrameType::ZRQINIT => {
                        self.state = RevcState::SendZRINIT;
                        return Ok(true);
                    }
                    FrameType::ZFILE => {
                        let pck = self.read_subpacket(com)?;
                        if pck.is_none() {
                            return Ok(false);
                        }
                        let (block, _) = pck.unwrap();


                        let file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
                        if self.files.len() == 0 || self.files.last().unwrap().file_name != file_name {
                            let mut fd =  FileDescriptor::new();
                            fd.data = Some(Vec::new());
                            fd.file_name = file_name;
                            transfer_state.write(format!("Got file header for '{}'", fd.file_name));
                            let mut file_size = 0;
                            for b in &block[(fd.file_name.len() + 1)..] {
                                if *b < b'0' || *b > b'9' {
                                    break;
                                }
                                file_size = file_size * 10 + (*b  - b'0') as usize;
                            }
                            fd.size = file_size;
                            self.files.push(fd);
                        }

                        self.state = RevcState::AwaitZDATA;
                        self.last_send = SystemTime::now();
                        self.request_zpos(com)?;
                        
                        return Ok(true);
                    }
                    FrameType::ZDATA => {
                        let offset = res.number();
                        if self.files.len() == 0 {
                            self.cancel(com)?;
                            return Err(io::Error::new(ErrorKind::InvalidInput, "Got ZDATA before ZFILE")); 
                        }
                        let last = self.files.len() - 1;
                        if let Some(fd) = self.files.get_mut(last) {
                            if let Some(data) = &mut fd.data {
                                if data.len() > offset as usize {
                                    data.resize(offset as usize, 0);
                                } else if data.len() < offset as usize {
                                    Header::from_number(HeaderType::Hex, FrameType::ZRPOS, data.len() as u32).write(com)?;
                                    return Ok(false);
                                }
                                self.state = RevcState::AwaitFileData;
                            }
                        }
                        return Ok(true);
                    }
                    FrameType::ZEOF => {
                        self.send_zrinit(com)?;
                        transfer_state.write("Got eof".to_string());
                        self.last_send = SystemTime::now();
                        self.state = RevcState::SendZRINIT;
                        return Ok(true);
                    }
                    FrameType::ZFIN => {
                        Header::empty(HeaderType::Hex, FrameType::ZFIN).write(com)?;
                        transfer_state.write("Transfer finished.".to_string());
                        self.state = RevcState::Idle;
                        return Ok(true);
                    }
                    unk_frame => {
                        return Err(io::Error::new(ErrorKind::InvalidInput, format!("unsupported frame {:?}.", unk_frame))); 
                    }
                }
            }
        }
        Ok(false)
    }


    pub fn read_subpacket(&mut self, com: &mut Box<dyn Com>) -> io::Result<Option<(Vec<u8>, bool)>>
    {
        let mut data = Vec::with_capacity(self.block_length);
        let d = Duration::from_secs(5);
        loop {
            let c = com.read_char(d)?;
            match c {
                ZDLE  => {
                    let c2 = com.read_char(d)?;
                    match c2 {
                        ZDLEE => data.push(ZDLE),
                        ESC_0X10 => data.push(0x10),
                        ESC_0X90 => data.push(0x90),
                        ESC_0X11 => data.push(0x11),
                        ESC_0X91 => data.push(0x91),
                        ESC_0X13 => data.push(0x13),
                        ESC_0X93 => data.push(0x93),
                        ESC_0X0D => data.push(0x0D),
                        ESC_0X8D => data.push(0x8D),
                        ZRUB0 => data.push(0x7F),
                        ZRUB1 => data.push(0xFF),
                        
                        ZCRCE => { // CRC next, frame ends, header packet follows 
                            if !self.check_crc(com, &data, c2)? { 
                                Header::empty(HeaderType::Bin32, FrameType::ZNAK).write(com)?;
                                return Ok(None); 
                            }
                            return Ok(Some((data, true)));
                        },
                        ZCRCG => { // CRC next, frame continues nonstop 
                            if !self.check_crc(com, &data, c2)? { 
                                Header::empty(HeaderType::Bin32, FrameType::ZNAK).write(com)?;
                                return Ok(None); 
                            }
                            return Ok(Some((data, false)));
                        },
                        ZCRCQ => { // CRC next, frame continues, ZACK expected
                            if !self.check_crc(com, &data, c2)? {
                                Header::empty(HeaderType::Bin32, FrameType::ZNAK).write(com)?;
                                return Ok(None);
                            }
                            Header::empty(HeaderType::Bin32, FrameType::ZACK).write(com)?;
                            return Ok(Some((data, false)));
                        },
                        ZCRCW => { // CRC next, ZACK expected, end of frame
                            if !self.check_crc(com, &data, c2)? { 
                                Header::empty(HeaderType::Bin32, FrameType::ZNAK).write(com)?;
                                return Ok(None);
                             }
                            Header::empty(HeaderType::Bin32, FrameType::ZACK).write(com)?;
                            return Ok(Some((data, true)));
                        },
                        _ => {
                            self.errors += 1;
                            println!("don't understand subpacket {}/x{:X}", c2, c2);
                            Header::empty(HeaderType::Bin32, FrameType::ZNAK).write(com)?;
                            return Ok(None);
                        }
                    }
                }
                0x11 | 0x91 | 0x13 | 0x93 => {
                    eprintln!("ignored byte");
                }
                _ => data.push(c)
            }
        }
    }

    fn check_crc(&mut self, com: &mut Box<dyn Com>, data: &Vec<u8>, zcrc_byte: u8)  -> io::Result<bool> {
        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);
        let crc_bytes = read_zdle_bytes(com, 4)?;
        let check_crc = u32::from_le_bytes(crc_bytes.try_into().unwrap());
        if crc == check_crc {
            Ok(true)
        } else {
            self.errors += 1;
            Header::empty(HeaderType::Bin32, FrameType::ZNAK).write(com)?;
            Ok(false)
        }
    }

    pub fn recv(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>
    {
        self.state = RevcState::Await;
        self.last_send = SystemTime::UNIX_EPOCH;
        self.retries = 0;
        self.send_zrinit(com)?;
        Ok(())
    }

    pub fn send_zrinit(&mut self, com: &mut Box<dyn Com>) -> io::Result<()> {
        Header::from_flags(HeaderType::Hex,FrameType::ZRINIT, 0, 0, 0, 0x23).write(com)?;
        Ok(())
    }
}


fn str_from_null_terminated_utf8_unchecked(s: &[u8]) -> &str {
    if let Ok(s) = std::ffi::CStr::from_bytes_until_nul(s) {
        s.to_str().unwrap()
    } else {
        ""
    }
}