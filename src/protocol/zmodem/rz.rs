#![allow(clippy::unused_self, clippy::wildcard_imports)]
use std::cmp::Ordering;

use icy_engine::{get_crc32, update_crc32};

use crate::{
    com::{Com, TermComResult},
    protocol::{
        str_from_null_terminated_utf8_unchecked, FileStorageHandler, Header, HeaderType,
        TransferState, ZFrameType, Zmodem, ZCRCE, ZCRCG, ZCRCW,
    },
};

use super::{constants::*, error_mod::TransmissionError, read_zdle_bytes};

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
            block_length,
            retries: 0,
            can_count: 0,
            errors: 0,
            sender_flags: 0,
            use_crc32: false,
        }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.state, RevcState::Idle)
    }

    fn get_header_type(&self) -> HeaderType {
        // does it make sense to value the flags from ZSINIT here?
        // Hex seems to be understood by all implementations and can be read by a human.
        // The receiver doesn't send large files so binary headers don't make much sense for the subpackets.
        HeaderType::Hex
    }

    fn cancel(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()> {
        self.state = RevcState::Idle;
        Zmodem::cancel(com)
    }

    pub fn update(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: &mut TransferState,
        storage_handler: &mut dyn FileStorageHandler,
    ) -> TermComResult<()> {
        if let RevcState::Idle = self.state {
            return Ok(());
        }
        if self.retries > 5 {
            self.cancel(com)?;
            return Ok(());
        }
        let transfer_info = &mut transfer_state.recieve_state;

        if let Some(file) = storage_handler.current_file_name() {
            transfer_info.file_name = file;
            transfer_info.file_size = storage_handler.get_current_file_total_size();
            transfer_info.bytes_transfered = storage_handler.current_file_length();
        }
        transfer_info.errors = self.errors;
        transfer_info.check_size = "Crc32".to_string();
        transfer_info.update_bps();

        match self.state {
            RevcState::SendZRINIT => {
                if self.read_header(com, storage_handler)? {
                    return Ok(());
                }
                /*  let now = SystemTime::now();
                 if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.send_zrinit(com)?;
                    self.retries += 1;
                    self.last_send = SystemTime::now();
                }*/
            } /*  identical to '_'
            RevcState::AwaitZDATA => {
            /*  let now = SystemTime::now();
                if now.duration_since(self.last_send).unwrap().as_millis() > 3000 {
                    self.request_zpos(com)?;
                    self.retries += 1;
                    self.last_send = SystemTime::now();
                }*/
                self.read_header(com)?;
            } */
            RevcState::AwaitFileData => {
                let pck = read_subpacket(com, self.block_length, self.use_crc32);
                if let Ok((block, is_last, expect_ack)) = pck {
                    if expect_ack {
                        Header::empty(self.get_header_type(), ZFrameType::Ack).write(com)?;
                    }
                    storage_handler.append(&block);
                    if is_last {
                        storage_handler.close();
                        self.state = RevcState::AwaitEOF;
                    }
                } else {
                    self.errors += 1;
                    //transfer_info.write(err.to_string());

                    if storage_handler.current_file_name().is_some() {
                        Header::from_number(
                            self.get_header_type(),
                            ZFrameType::RPos,
                            u32::try_from(storage_handler.current_file_length()).unwrap(),
                        )
                        .write(com)?;
                        self.state = RevcState::AwaitZDATA;
                    }
                    return Ok(());
                }
            }
            _ => {
                self.read_header(com, storage_handler)?;
            }
        }
        Ok(())
    }

    fn request_zpos(&mut self, com: &mut Box<dyn Com>) -> TermComResult<usize> {
        Header::from_number(self.get_header_type(), ZFrameType::RPos, 0).write(com)
    }

    fn read_header(
        &mut self,
        com: &mut Box<dyn Com>,
        storage_handler: &mut dyn FileStorageHandler,
    ) -> TermComResult<bool> {
        let result = Header::read(com, &mut self.can_count);
        if result.is_err() {
            if self.can_count >= 5 {
                //transfer_state.write("Received cancel...".to_string());
                self.cancel(com)?;
                self.cancel(com)?;
                self.cancel(com)?;
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
            println!("\t\t\t\t\t\tRECV header {res}");
            self.use_crc32 = res.header_type == HeaderType::Bin32;
            match res.frame_type {
                ZFrameType::Sinit => {
                    let pck = read_subpacket(com, self.block_length, self.use_crc32);
                    if pck.is_err() {
                        Header::empty(self.get_header_type(), ZFrameType::Nak).write(com)?;
                        return Ok(false);
                    }
                    // TODO: Atn sequence
                    self.sender_flags = res.f0();
                    Header::empty(self.get_header_type(), ZFrameType::Ack).write(com)?;
                    return Ok(true);
                }

                ZFrameType::RQInit => {
                    self.state = RevcState::SendZRINIT;
                    return Ok(true);
                }
                ZFrameType::File => {
                    let pck = read_subpacket(com, self.block_length, self.use_crc32);

                    if let Ok((block, _, _)) = pck {
                        let file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
                        let mut file_size = 0;
                        for b in &block[(file_name.len() + 1)..] {
                            if *b < b'0' || *b > b'9' {
                                break;
                            }
                            file_size = file_size * 10 + (*b - b'0') as usize;
                        }
                        storage_handler.open_file(&file_name, file_size);

                        self.state = RevcState::AwaitZDATA;
                        self.request_zpos(com)?;

                        return Ok(true);
                    }
                    self.errors += 1;
                    //transfer_state.write(format!("Got no ZFILE subpacket: {}", err));
                    return Ok(false);
                }
                ZFrameType::Data => {
                    let offset = res.number();
                    if storage_handler.current_file_name().is_none() {
                        self.cancel(com)?;
                        return Err(Box::new(TransmissionError::ZDataBeforeZFILE));
                    }
                    let header_type = self.get_header_type();
                    let len = storage_handler.current_file_length();
                    match len.cmp(&(offset as usize)) {
                        Ordering::Greater => storage_handler.set_current_size_to(offset as usize),
                        Ordering::Less => {
                            Header::from_number(header_type, ZFrameType::RPos, len as u32)
                                .write(com)?;
                            return Ok(false);
                        }
                        Ordering::Equal => {}
                    }
                    self.state = RevcState::AwaitFileData;
                    return Ok(true);
                }
                ZFrameType::Eof => {
                    self.send_zrinit(com)?;
                    storage_handler.close();
                    //transfer_state.write("Got eof".to_string());
                    self.state = RevcState::SendZRINIT;
                    return Ok(true);
                }
                ZFrameType::Fin => {
                    Header::empty(self.get_header_type(), ZFrameType::Fin).write(com)?;
                    //transfer_state.write("Transfer finished.".to_string());
                    self.state = RevcState::Idle;
                    return Ok(true);
                }
                ZFrameType::Challenge => {
                    // isn't specfied for receiver side.
                    Header::from_number(self.get_header_type(), ZFrameType::Ack, res.number())
                        .write(com)?;
                }
                ZFrameType::FreeCnt => {
                    // 0 means unlimited space but sending free hd space to an unknown source is a security issue
                    Header::from_number(self.get_header_type(), ZFrameType::Ack, 0).write(com)?;
                }
                ZFrameType::Command => {
                    // just protocol it.
                    let package = read_subpacket(com, self.block_length, self.use_crc32);
                    if let Ok((block, _, _)) = &package {
                        let cmd = str_from_null_terminated_utf8_unchecked(block);
                        eprintln!(
                            "Remote wanted to execute {cmd} on the system. (did not execute)"
                        );
                    }
                    Header::from_number(self.get_header_type(), ZFrameType::Compl, 0).write(com)?;
                }
                ZFrameType::Abort | ZFrameType::FErr | ZFrameType::Can => {
                    Header::empty(self.get_header_type(), ZFrameType::Fin).write(com)?;
                    self.state = RevcState::Idle;
                }
                unk_frame => {
                    return Err(Box::new(TransmissionError::UnsupportedFrame(unk_frame)));
                }
            }
        }
        Ok(false)
    }

    pub fn recv(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()> {
        self.state = RevcState::Await;
        self.retries = 0;
        self.send_zrinit(com)?;
        Ok(())
    }

    pub fn send_zrinit(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()> {
        Header::from_flags(self.get_header_type(), ZFrameType::RIinit, 0, 0, 0, 0x23).write(com)?;
        Ok(())
    }
}

pub fn read_subpacket(
    com: &mut Box<dyn Com>,
    block_length: usize,
    use_crc32: bool,
) -> TermComResult<(Vec<u8>, bool, bool)> {
    let mut data = Vec::with_capacity(block_length);
    loop {
        let c = com.read_u8()?;
        match c {
            ZDLE => {
                let c2 = com.read_u8()?;
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
                        check_crc(com, use_crc32, &data, c2)?;
                        return Ok((data, true, false));
                    }
                    ZCRCG => {
                        // CRC next, frame continues nonstop
                        check_crc(com, use_crc32, &data, c2)?;
                        return Ok((data, false, false));
                    }
                    ZCRCQ => {
                        // CRC next, frame continues, ZACK expected
                        check_crc(com, use_crc32, &data, c2)?;
                        return Ok((data, false, true));
                    }
                    ZCRCW => {
                        // CRC next, ZACK expected, end of frame
                        check_crc(com, use_crc32, &data, c2)?;
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

fn check_crc(
    com: &mut Box<dyn Com>,
    use_crc32: bool,
    data: &[u8],
    zcrc_byte: u8,
) -> TermComResult<bool> {
    if use_crc32 {
        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);
        let crc_bytes = read_zdle_bytes(com, 4)?;
        let check_crc = u32::from_le_bytes(crc_bytes.try_into().unwrap());
        if crc == check_crc {
            Ok(true)
        } else {
            Err(Box::new(TransmissionError::CRC32Mismatch(crc, check_crc)))
        }
    } else {
        let crc = icy_engine::get_crc16_buggy(data, zcrc_byte);
        let crc_bytes = read_zdle_bytes(com, 2)?;
        let check_crc = u16::from_le_bytes(crc_bytes.try_into().unwrap());
        if crc == check_crc {
            Ok(true)
        } else {
            Err(Box::new(TransmissionError::CRC16Mismatch(crc, check_crc)))
        }
    }
}
