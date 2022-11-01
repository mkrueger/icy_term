// 
// ZModem protocol specification http://cristal.inria.fr/~doligez/zmodem/zmodem.txt

pub mod constants;
use std::io;

pub use constants::*;
mod header;
pub use header::*;

mod sz;
use icy_engine::{get_crc16, get_crc32, update_crc32, update_crc16};
use sz::*;

mod rz;
use rz::*;

mod tests;

use crate::{com::Com};
use super::{Protocol, TransferState, FileTransferState};

pub struct Zmodem {
    block_length: usize,
    transfer_state: Option<TransferState>,
    sz: Sz,
    rz: Rz
}

impl Zmodem {
    pub fn new(block_length: usize) -> Self {
        Self {
            block_length,
            transfer_state: None,
            sz: Sz::new(block_length),
            rz: Rz::new()
        }
    }

    pub fn cancel<T: Com>(com: &mut T) -> io::Result<()> {
        com.write(&ABORT_SEQ)?;
        Ok(())
    }

    pub fn _encode_subpacket_crc16(zcrc_byte: u8, data:&[u8]) -> Vec<u8>
    {
        let mut v = Vec::new();

        let mut crc = get_crc16(data);
        crc = update_crc16(crc, zcrc_byte);

        append_escape(&mut v, data);
        append_escape(&mut v, &[ZDLE, zcrc_byte]);
        v.extend_from_slice(&u16::to_le_bytes(crc));
        v
    }

    pub fn encode_subpacket_crc32(zcrc_byte: u8, data:&[u8]) -> Vec<u8>
    {
        let mut v = Vec::new();

        let mut crc = get_crc32(data);
        crc = !update_crc32(!crc, zcrc_byte);

        append_escape(&mut v, data);
        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_escape(&mut v, &u32::to_le_bytes(crc));
        v
    }
}



fn append_escape(v: &mut Vec<u8>, data: &[u8])
{
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
            },
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

fn get_hex(n: u8) -> u8
{
    if n < 10 {
        return b'0' + n as u8;
    }
    return b'a' + (n - 10) as u8;
}

fn from_hex(n: u8) -> io::Result<u8>
{
    if b'0' <= n && n <= b'9' {
        return Ok(n - b'0');
    }
    if b'A' <= n && n <= b'F' {
        return Ok(10 + n - b'A');
    }
    if b'a' <= n && n <= b'f' {
        return Ok(10 + n - b'a');
    }
    return Err(io::Error::new(io::ErrorKind::InvalidData, "Hex number expected"));
}

impl Protocol for Zmodem {

    fn get_name(&self) -> &str
    {
        if self.block_length == 1024 { "Zmodem" } else { "ZedZap (Zmodem 8k)" }
    }

    fn get_current_state(&self) -> Option<&TransferState>
    {
       self.transfer_state.as_ref()
    }

    fn is_active(&self) -> bool
    {
        self.transfer_state.is_some()
    }

    fn update<T: crate::com::Com>(&mut self, com: &mut T) -> std::io::Result<()> {
        match &mut self.transfer_state  {
            Some(s) => {
                if self.sz.is_active() {
                    self.sz.update(com, s)?;
                    if !self.sz.is_active() {
                        self.transfer_state = None;
                    }
                } else if self.rz.is_active() {
                    self.rz.update(com, s)?;
                    if !self.rz.is_active() {
                        self.transfer_state = None;
                    }
                }
            }
            None => {
                println!("no state");
                return Ok(());
            }
        }
        
        Ok(())
    }

    fn initiate_send<T: crate::com::Com>(&mut self, com: &mut T, files: Vec<super::FileDescriptor>) -> std::io::Result<()> {
        let mut state = TransferState::new();
        state.send_state = Some(FileTransferState::new());
        self.transfer_state = Some(state);

        self.sz.send(com, files)
    }

    fn initiate_recv<T: crate::com::Com>(&mut self, com: &mut T) -> std::io::Result<()> {
        let mut state = TransferState::new();
        state.recieve_state = Some(FileTransferState::new());
        self.transfer_state = Some(state);

        self.rz.recv(com)
    }

    fn get_received_files(&mut self) -> Vec<super::FileDescriptor> {
        let c = self.rz.files.clone();
        self.rz.files = Vec::new();
        c
    }

    fn cancel<T: crate::com::Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.transfer_state = None;
        com.write(&ABORT_SEQ)?;
        com.write(&ABORT_SEQ)?;
        Ok(())
    }

}

