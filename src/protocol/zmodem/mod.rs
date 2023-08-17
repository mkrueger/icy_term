//
// ZModem protocol specification http://cristal.inria.fr/~doligez/zmodem/zmodem.txt

pub mod constants;

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

use self::{error_mod::TransmissionError, rz::read_zdle_byte};

use super::{FileDescriptor, FileStorageHandler, Protocol, TransferState};
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

    pub fn encode_subpacket_crc16(zcrc_byte: u8, data: &[u8], escape_ctl_chars: bool) -> Vec<u8> {
        let mut v = Vec::new();
        let crc = icy_engine::get_crc16_buggy(data, zcrc_byte);
        append_zdle_encoded(&mut v, data, escape_ctl_chars);
        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_zdle_encoded(&mut v, &u16::to_le_bytes(crc), escape_ctl_chars);
        v
    }

    pub fn encode_subpacket_crc32(zcrc_byte: u8, data: &[u8], escape_ctl_chars: bool) -> Vec<u8> {
        let mut v = Vec::new();
        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);

        append_zdle_encoded(&mut v, data, escape_ctl_chars);
        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_zdle_encoded(&mut v, &u32::to_le_bytes(crc), escape_ctl_chars);
        v
    }
}

pub fn append_zdle_encoded(v: &mut Vec<u8>, data: &[u8], escape_ctl_chars: bool) {
    let mut last = 0u8;
    for b in data {
        match *b {
            DLE | DLE_0x80 | XON | XON_0x80 | XOFF | XOFF_0x80 | ZDLE => {
                v.extend_from_slice(&[ZDLE, *b ^ 0x40]);
            }
            CR | CR_0x80 => {
                if escape_ctl_chars && last == b'@' {
                    v.extend_from_slice(&[ZDLE, *b ^ 0x40]);
                } else {
                    v.push(*b);
                }
            }

            b => {
                if escape_ctl_chars && (b & 0x60) == 0 {
                    v.extend_from_slice(&[ZDLE, b ^ 0x40]);
                } else {
                    v.push(b);
                }
            }
        }
        last = *b;
    }
}

pub fn read_zdle_bytes(com: &mut Box<dyn Com>, length: usize) -> TermComResult<Vec<u8>> {
    let mut data = Vec::new();
    for _ in 0..length {
        let c = read_zdle_byte(com, false)?;
        if let rz::ZModemResult::Ok(b) = c {
            data.push(b);
        }
    }
    Ok(data)
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
        transfer_state: &mut TransferState,
        storage_handler: &mut dyn FileStorageHandler,
    ) -> TermComResult<bool> {
        if let Some(rz) = &mut self.rz {
            rz.update(com, transfer_state, storage_handler)?;
            if !rz.is_active() {
                transfer_state.is_finished = true;
                return Ok(false);
            }
        } else if let Some(sz) = &mut self.sz {
            sz.update(com, transfer_state)?;
            if !sz.is_active() {
                transfer_state.is_finished = true;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn initiate_send(
        &mut self,
        com: &mut Box<dyn Com>,
        files: Vec<FileDescriptor>,
        transfer_state: &mut TransferState,
    ) -> TermComResult<()> {
        transfer_state.protocol_name = self.get_name().to_string();
        let mut sz = Sz::new(self.block_length);
        sz.send(com, files);
        self.sz = Some(sz);
        Ok(())
    }

    fn initiate_recv(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: &mut TransferState,
    ) -> TermComResult<()> {
        transfer_state.protocol_name = self.get_name().to_string();
        let mut rz = Rz::new(self.block_length);
        rz.recv(com)?;
        self.rz = Some(rz);
        Ok(())
    }

    fn cancel(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()> {
        com.send(&ABORT_SEQ)?;
        Ok(())
    }
}
