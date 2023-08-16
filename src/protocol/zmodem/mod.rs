//
// ZModem protocol specification http://cristal.inria.fr/~doligez/zmodem/zmodem.txt

pub mod constants;
use std::sync::{Arc, Mutex};

pub use constants::*;
mod header_mod;
pub use header_mod::*;
use icy_engine::{get_crc32, update_crc32};

mod sz;
use sz::Sz;

mod rz;
use rz::Rz;

mod error_mod;
mod tests;

use self::error_mod::TransmissionError;

use super::{FileDescriptor, Protocol, TransferState};
use crate::com::{Com, TermComResult};

pub struct Zmodem {
    block_length: usize,
    rz: Option<rz::Rz>,
    sz: Option<sz::Sz>,
}

impl Zmodem {
    pub fn new(block_length: usize) -> Self {
        Self {
            block_length,
            sz: None,
            rz: None,
        }
    }

    fn get_name(&self) -> &str {
        if self.block_length == 1024 {
            "Zmodem"
        } else {
            "ZedZap (Zmodem 8k)"
        }
    }

    pub fn cancel(com: &mut Box<dyn Com>) -> TermComResult<()> {
        com.send(&ABORT_SEQ)?;
        Ok(())
    }

    pub fn encode_subpacket_crc16(zcrc_byte: u8, data: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        let crc = icy_engine::get_crc16_buggy(data, zcrc_byte);
        append_zdle_encoded(&mut v, data);
        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_zdle_encoded(&mut v, &u16::to_le_bytes(crc));
        v
    }

    pub fn encode_subpacket_crc32(zcrc_byte: u8, data: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);

        append_zdle_encoded(&mut v, data);
        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_zdle_encoded(&mut v, &u32::to_le_bytes(crc));
        v
    }
}

pub fn append_zdle_encoded(v: &mut Vec<u8>, data: &[u8]) {
    let mut last = 0u8;
    for b in data {
        match *b {
            ZDLE => v.extend_from_slice(&[ZDLE, ZDLEE]),
            0x10 => v.extend_from_slice(&[ZDLE, ESC_0X10]),
            0x90 => v.extend_from_slice(&[ZDLE, ESC_0X90]),
            0x11 => v.extend_from_slice(&[ZDLE, ESC_0X11]),
            0x91 => v.extend_from_slice(&[ZDLE, ESC_0X91]),
            0x13 => v.extend_from_slice(&[ZDLE, ESC_0X13]),
            0x93 => v.extend_from_slice(&[ZDLE, ESC_0X93]),
            0x0D => {
                if last == 0x40 || last == 0xc0 {
                    v.extend_from_slice(&[ZDLE, ESC_0X0D]);
                } else {
                    v.push(0x0D);
                }
            }
            0x8D => {
                if last == 0x40 || last == 0xc0 {
                    v.extend_from_slice(&[ZDLE, ESC_0X8D]);
                } else {
                    v.push(0x8D);
                }
            }

            b => v.push(b),
        }
        last = *b;
    }
}

pub fn read_zdle_bytes(com: &mut Box<dyn Com>, length: usize) -> TermComResult<Vec<u8>> {
    let mut data = Vec::new();
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

                    _ => {
                        Header::empty(HeaderType::Bin32, ZFrameType::Nak).write(com)?;
                        return Err(Box::new(TransmissionError::InvalidSubpacket(c2)));
                    }
                }
            }
            0x11 | 0x91 | 0x13 | 0x93 => {
                // println!("ignored byte");
            }
            _ => {
                data.push(c);
            }
        }
        if data.len() >= length {
            return Ok(data);
        }
    }
}

fn get_hex(n: u8) -> u8 {
    if n < 10 {
        b'0' + n
    } else {
        b'a' + (n - 10)
    }
}

fn from_hex(n: u8) -> TermComResult<u8> {
    if n.is_ascii_digit() {
        return Ok(n - b'0');
    }
    if (b'A'..=b'F').contains(&n) {
        return Ok(10 + n - b'A');
    }
    if (b'a'..=b'f').contains(&n) {
        return Ok(10 + n - b'a');
    }
    Err(Box::new(TransmissionError::HexNumberExpected))
}

impl Protocol for Zmodem {
    fn update(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: Arc<Mutex<TransferState>>,
    ) -> TermComResult<bool> {
        if let Some(rz) = &mut self.rz {
            rz.update(com, &transfer_state)?;
            if !rz.is_active() {
                transfer_state.lock().unwrap().is_finished = true;
                return Ok(false);
            }
        } else if let Some(sz) = &mut self.sz {
            sz.update(com, &transfer_state)?;
            if !sz.is_active() {
                transfer_state.lock().unwrap().is_finished = true;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn initiate_send(
        &mut self,
        com: &mut Box<dyn Com>,
        files: Vec<FileDescriptor>,
        transfer_state: Arc<Mutex<TransferState>>,
    ) -> TermComResult<()> {
        transfer_state.lock().unwrap().protocol_name = self.get_name().to_string();
        let mut sz = Sz::new(self.block_length);
        sz.send(com, files);
        self.sz = Some(sz);
        Ok(())
    }

    fn initiate_recv(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: Arc<Mutex<TransferState>>,
    ) -> TermComResult<()> {
        transfer_state.lock().unwrap().protocol_name = self.get_name().to_string();
        let mut rz = Rz::new(self.block_length);
        rz.recv(com)?;
        self.rz = Some(rz);
        Ok(())
    }

    fn get_received_files(&mut self) -> Vec<super::FileDescriptor> {
        if let Some(rz) = &mut self.rz {
            let c = rz.files.clone();
            rz.files = Vec::new();
            c
        } else {
            Vec::new()
        }
    }

    fn cancel(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()> {
        com.send(&ABORT_SEQ)?;
        Ok(())
    }
}
