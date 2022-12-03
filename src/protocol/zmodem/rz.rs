use std::{
    time::{SystemTime}, sync::{Arc, Mutex}, fs,
};

use directories::UserDirs;
use icy_engine::{get_crc32, update_crc32};

use crate::{
    protocol::{
        str_from_null_terminated_utf8_unchecked, FileDescriptor, FrameType,
        Header, HeaderType, TransferState, Zmodem, ZCRCE, ZCRCG, ZCRCW,
    }, com::{Com, ComResult}, 
};

use super::{constants::*, read_zdle_bytes, error::TransmissionError};

#[derive(Debug)]
pub enum RevcState {
    Idle,
    Await,
    AwaitZDATA,
    AwaitFileData,
    AwaitEOF,
    SendZRINIT,
}

pub struct Rz {
    state: RevcState,
    pub files: Vec<FileDescriptor>,
    pub errors: usize,
    retries: usize,
    can_count: usize,
    block_length: usize,
    sender_flags: u8,
    use_crc32: bool,
}

impl Rz {
    pub fn new(block_length: usize) -> Self {
        Self {
            state: RevcState::Idle,
            files: Vec::new(),
            block_length,
            retries: 0,
            can_count: 0,
            errors: 0,
            sender_flags: 0,
            use_crc32: false,
        }
    }

    pub fn is_active(&self) -> bool {
        if let RevcState::Idle = self.state {
            false
        } else {
            true
        }
    }

    fn get_header_type(&self) -> HeaderType {
        // does it make sense to value the flags from ZSINIT here?
        // Hex seems to be understood by all implementations and can be read by a human.
        // The receiver doesn't send large files so binary headers don't make much sense for the subpackets.
        HeaderType::Hex
    }

    async fn cancel(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        self.state = RevcState::Idle;
        Zmodem::cancel(com).await
    }

    pub async fn update(&mut self, com: &mut Box<dyn Com>, transfer_state: Arc<Mutex<TransferState>>) -> ComResult<()> {
        if let RevcState::Idle = self.state {
            return Ok(());
        }
        if self.retries > 5 {
            self.cancel(com).await?;
            return Ok(());
        }
        if let Ok(transfer_state) = &mut transfer_state.lock() {
            let transfer_info = &mut transfer_state.recieve_state;

            if self.files.len() > 0 {
                let cur_file = self.files.len() - 1;
                let fd = &self.files[cur_file];
                transfer_info.file_name = fd.file_name.clone();
                transfer_info.file_size = fd.size;
                transfer_info.bytes_transfered = fd.data.as_ref().unwrap().len();
            }
            transfer_info.errors = self.errors;
            transfer_info.check_size = format!("Crc32");
            transfer_info.update_bps();
        }

        match self.state {
            RevcState::SendZRINIT => {
                if self.read_header(com).await? {
                    return Ok(());
                }
                let now = SystemTime::now();
              /*   if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zrinit(com).await?;
                    self.retries += 1;
                    self.last_send = SystemTime::now();
                }*/
            }
            RevcState::AwaitZDATA => {
                let now = SystemTime::now();
               /* if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.request_zpos(com).await?;
                    self.retries += 1;
                    self.last_send = SystemTime::now();
                }*/
                self.read_header(com).await?;
            }
            RevcState::AwaitFileData => {
                let pck = read_subpacket(com, self.block_length, self.use_crc32).await;
                let last = self.files.len() - 1;
                match pck {
                    Ok((block, is_last, expect_ack)) => {
                        if expect_ack {
                            Header::empty(self.get_header_type(), FrameType::ZACK)
                                .write(com).await?;
                        }
                        if let Some(fd) = self.files.get_mut(last) {
                            if let Some(data) = &mut fd.data {
                                data.extend_from_slice(&block);
                            }
                        }
                        if is_last {
                            self.state = RevcState::AwaitEOF;
                        }
                    }
                    Err(err) => {
                        self.errors += 1;
                        //transfer_info.write(err.to_string());

                        if let Some(fd) = self.files.get(last) {
                            Header::from_number(
                                self.get_header_type(),
                                FrameType::ZRPOS,
                                fd.data.as_ref().unwrap().len() as u32,
                            )
                            .write(com).await?;
                            self.state = RevcState::AwaitZDATA;
                        }
                        return Ok(());
                    }
                }
            }
            _ => {
                self.read_header(com).await?;
            }
        }
        Ok(())
    }

    async fn request_zpos(&mut self, com: &mut Box<dyn Com>) -> ComResult<usize> {
        Header::from_number(self.get_header_type(), FrameType::ZRPOS, 0).write(com).await
    }

    async fn read_header(
        &mut self,
        com: &mut Box<dyn Com>
    ) -> ComResult<bool> {
        let result = Header::read(com, &mut self.can_count).await;
        if let Err(err) = result {
            if self.can_count >= 5 {
                //transfer_state.write("Received cancel...".to_string());
                self.cancel(com).await?;
                self.cancel(com).await?;
                self.cancel(com).await?;
                self.state = RevcState::Idle;
                return Ok(false);
            }
            //transfer_state.write(format!("{}", err));
            self.errors += 1;
            return Ok(false);
        }
        self.can_count = 0;
        let res = result?;
        if let Some(res) = res {
            // println!("\t\t\t\t\t\tRECV header {}", res);
            self.use_crc32 = res.header_type == HeaderType::Bin32;
            match res.frame_type {
                FrameType::ZSINIT => {
                    let pck = read_subpacket(com, self.block_length, self.use_crc32).await;
                    if pck.is_err() {
                        Header::empty(self.get_header_type(), FrameType::ZNAK).write(com).await?;
                        return Ok(false);
                    }
                    // TODO: Atn sequence
                    self.sender_flags = res.f0();
                    Header::empty(self.get_header_type(), FrameType::ZACK).write(com).await?;
                    return Ok(true);
                }

                FrameType::ZRQINIT => {
                    self.state = RevcState::SendZRINIT;
                    return Ok(true);
                }
                FrameType::ZFILE => {
                    let pck = read_subpacket(com, self.block_length, self.use_crc32).await;

                    match pck {
                        Ok((block, _, _)) => {
                            let file_name =
                                str_from_null_terminated_utf8_unchecked(&block).to_string();
                            if self.files.len() == 0
                                || self.files.last().unwrap().file_name != file_name
                            {
                                
                                let mut fd = FileDescriptor::new();
                                fd.data = Some(Vec::new());
                                fd.file_name = file_name;
                                //transfer_state.write(format!("Got file header for '{}'", fd.file_name));
                                let mut file_size = 0;
                                for b in &block[(fd.file_name.len() + 1)..] {
                                    if *b < b'0' || *b > b'9' {
                                        break;
                                    }
                                    file_size = file_size * 10 + (*b - b'0') as usize;
                                }
                                fd.size = file_size;
                                self.files.push(fd);
                            }

                            self.state = RevcState::AwaitZDATA;
                            self.request_zpos(com).await?;

                            return Ok(true);
                        }
                        Err(err) => {
                            self.errors += 1;
                            //transfer_state.write(format!("Got no ZFILE subpacket: {}", err));
                            return Ok(false);
                        }
                    }
                }
                FrameType::ZDATA => {
                    let offset = res.number();
                    if self.files.len() == 0 {
                        self.cancel(com).await?;
                        return Err(Box::new(TransmissionError::ZDataBeforeZFILE));
                    }
                    let header_type = self.get_header_type();
                    let last = self.files.len() - 1;
                    if let Some(fd) = self.files.get_mut(last) {
                        if let Some(data) = &mut fd.data {
                            if data.len() > offset as usize {
                                data.resize(offset as usize, 0);
                            } else if data.len() < offset as usize {
                                Header::from_number(
                                    header_type,
                                    FrameType::ZRPOS,
                                    data.len() as u32,
                                )
                                .write(com).await?;
                                return Ok(false);
                            }
                            self.state = RevcState::AwaitFileData;
                        }
                    }
                    return Ok(true);
                }
                FrameType::ZEOF => {
                    self.send_zrinit(com).await?;
                    self.save_last_file()?;
                    //transfer_state.write("Got eof".to_string());
                    self.state = RevcState::SendZRINIT;
                    return Ok(true);
                }
                FrameType::ZFIN => {
                    Header::empty(self.get_header_type(), FrameType::ZFIN).write(com).await?;
                    //transfer_state.write("Transfer finished.".to_string());
                    self.state = RevcState::Idle;
                    return Ok(true);
                }
                FrameType::ZCHALLENGE => {
                    // isn't specfied for receiver side.
                    Header::from_number(self.get_header_type(), FrameType::ZACK, res.number())
                        .write(com).await?;
                }
                FrameType::ZFREECNT => {
                    // 0 means unlimited space but sending free hd space to an unknown source is a security issue
                    Header::from_number(self.get_header_type(), FrameType::ZACK, 0)
                        .write(com).await?;
                }
                FrameType::ZCOMMAND => {
                    // just protocol it.
                    let package = read_subpacket(com, self.block_length, self.use_crc32).await;
                    if let Ok((block, _, _)) = &package {
                        let cmd = str_from_null_terminated_utf8_unchecked(&block).to_string();
                        eprintln!(
                            "Remote wanted to execute {} on the system. (did not execute)",
                            cmd
                        );
                    }
                    Header::from_number(self.get_header_type(), FrameType::ZCOMPL, 0)
                        .write(com).await?;
                }
                FrameType::ZABORT | FrameType::ZFERR | FrameType::ZCAN => {
                    Header::empty(self.get_header_type(), FrameType::ZFIN).write(com).await?;
                    self.state = RevcState::Idle;
                }
                unk_frame => {
                    return Err(Box::new(TransmissionError::UnsupportedFrame(unk_frame)));

                }
            }
        }
        Ok(false)
    }

    fn save_last_file(&mut self) -> ComResult<()> {
        if self.files.len() > 0 {
            let fd = self.files.last().unwrap();

            if let Some(user_dirs) = UserDirs::new() {
                let dir = user_dirs.download_dir().unwrap();
            
                let mut file_name = dir.join(&fd.file_name);
                let mut i = 1;
                while file_name.exists() {
                    file_name = dir.join(&format!("{}.{}", fd.file_name, i));
                    i += 1;
                }
                fs::write(file_name, fd.data.as_ref().unwrap())?;
            }
        }

        Ok(())
    }

    pub async fn recv(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        self.state = RevcState::Await;
        self.retries = 0;
        self.send_zrinit(com).await?;
        Ok(())
    }

    pub async fn send_zrinit(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        Header::from_flags(self.get_header_type(), FrameType::ZRINIT, 0, 0, 0, 0x23).write(com).await?;
        Ok(())
    }
}

pub async fn read_subpacket(
    com: &mut Box<dyn Com>,
    block_length: usize,
    use_crc32: bool,
) -> ComResult<(Vec<u8>, bool, bool)> {
    let mut data = Vec::with_capacity(block_length);
    loop {
        let c = com.read_u8().await?;
        match c {
            ZDLE => {
                let c2 = com.read_u8().await?;
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

                    ZCRCE => {
                        // CRC next, frame ends, header packet follows
                        check_crc(com, use_crc32, &data, c2).await?;
                        return Ok((data, true, false));
                    }
                    ZCRCG => {
                        // CRC next, frame continues nonstop
                        check_crc(com, use_crc32, &data, c2).await?;
                        return Ok((data, false, false));
                    }
                    ZCRCQ => {
                        // CRC next, frame continues, ZACK expected
                        check_crc(com, use_crc32, &data, c2).await?;
                        return Ok((data, false, true));
                    }
                    ZCRCW => {
                        // CRC next, ZACK expected, end of frame
                        check_crc(com, use_crc32, &data, c2).await?;
                        return Ok((data, true, true));
                    }
                    _ => {
                        return Err(Box::new(TransmissionError::InvalidSubpacket(c2)));
                    }
                }
            }
            0x11 | 0x91 | 0x13 | 0x93 => {
                // they should be ignored, not errored according to spec
                eprintln!("ignored byte");
            }
            _ => data.push(c),
        }
    }
}

async fn check_crc(
    com: &mut Box<dyn Com>,
    use_crc32: bool,
    data: &Vec<u8>,
    zcrc_byte: u8,
) -> ComResult<bool> {
    if use_crc32 {
        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);
        let crc_bytes = read_zdle_bytes(com, 4).await?;
        let check_crc = u32::from_le_bytes(crc_bytes.try_into().unwrap());
        if crc == check_crc {
            Ok(true)
        } else {
            Err(Box::new(TransmissionError::CRC32Mismatch(crc, check_crc)))
        }
    } else {
        let crc = icy_engine::get_crc16_buggy(data, zcrc_byte);
        let crc_bytes = read_zdle_bytes(com, 2).await?;
        let check_crc = u16::from_le_bytes(crc_bytes.try_into().unwrap());
        if crc == check_crc {
            Ok(true)
        } else {
            Err(Box::new(TransmissionError::CRC16Mismatch(crc, check_crc)))
        }
    }
}
