//
// ZModem protocol specification http://cristal.inria.fr/~doligez/zmodem/zmodem.txt

pub mod constants;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
pub use constants::*;
mod header;
pub use header::*;
use icy_engine::{get_crc32, update_crc32};

mod sz;
use sz::*;

mod rz;
use rz::*;

mod error;
mod tests;

use self::error::TransmissionError;

use super::{FileDescriptor, Protocol, TransferState};
use crate::com::{Com, ComResult};

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

    pub async fn cancel(com: &mut Box<dyn Com>) -> ComResult<()> {
        com.send(&ABORT_SEQ).await?;
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

pub async fn read_zdle_bytes(com: &mut Box<dyn Com>, length: usize) -> ComResult<Vec<u8>> {
    let mut data = Vec::new();
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

                    _ => {
                        Header::empty(HeaderType::Bin32, FrameType::ZNAK)
                            .write(com)
                            .await?;
                        return Err(Box::new(TransmissionError::InvalidSubpacket(c2)));
                    }
                }
            }
            0x11 | 0x91 | 0x13 | 0x93 => {
                println!("ignored byte");
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
        return b'0' + n as u8;
    }
    return b'a' + (n - 10) as u8;
}

fn from_hex(n: u8) -> ComResult<u8> {
    if b'0' <= n && n <= b'9' {
        return Ok(n - b'0');
    }
    if b'A' <= n && n <= b'F' {
        return Ok(10 + n - b'A');
    }
    if b'a' <= n && n <= b'f' {
        return Ok(10 + n - b'a');
    }
    return Err(Box::new(TransmissionError::HexNumberExpected));
}

#[async_trait]
impl Protocol for Zmodem {
    async fn update(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: Arc<Mutex<TransferState>>,
    ) -> ComResult<bool> {
        if let Some(rz) = &mut self.rz {
            rz.update(com, transfer_state.clone()).await?;
            if !rz.is_active() {
                transfer_state.lock().unwrap().is_finished = true;
                return Ok(false);
            }
        } else if let Some(sz) = &mut self.sz {
            sz.update(com, transfer_state.clone()).await?;
            if !sz.is_active() {
                transfer_state.lock().unwrap().is_finished = true;
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn initiate_send(
        &mut self,
        com: &mut Box<dyn Com>,
        files: Vec<FileDescriptor>,
        transfer_state: Arc<Mutex<TransferState>>,
    ) -> ComResult<()> {
        transfer_state.lock().unwrap().protocol_name = self.get_name().to_string();
        let mut sz = Sz::new(self.block_length);
        sz.send(com, files).await?;
        self.sz = Some(sz);
        Ok(())
    }

    async fn initiate_recv(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: Arc<Mutex<TransferState>>,
    ) -> ComResult<()> {
        transfer_state.lock().unwrap().protocol_name = self.get_name().to_string();
        let mut rz = Rz::new(self.block_length);
        rz.recv(com).await?;
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

    async fn cancel(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        com.send(&ABORT_SEQ).await?;
        Ok(())
    }
}
