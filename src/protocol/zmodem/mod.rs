// 
// ZModem protocol specification http://cristal.inria.fr/~doligez/zmodem/zmodem.txt

pub mod constants;
use std::io;

pub use constants::*;
mod header;
pub use header::*;

mod sz;
use sz::*;

use crate::{crc16, com::Com};

use super::{Protocol, TransferState};

pub struct Zmodem {
    transfer_state: Option<TransferState>,
    sz: Sz
}

impl Zmodem {
    pub fn new() -> Self {
        Self {
            transfer_state: None,
            sz: Sz::new()
        }
    }

    pub fn is_active(&self) -> bool
    {
        self.sz.is_active()
    }
    pub fn cancel<T: Com>(com: &mut T) -> io::Result<()> {
        com.write(&ABORT_SEQ)?;
        Ok(())
    }


    pub fn _encode_subpacket_crc16(zcrc_byte: u8, data:&[u8]) -> Vec<u8>
    {
        let mut v = Vec::new();

        let mut crc = crc16::get_crc16(data);
        crc = crc16::update_crc16(crc, zcrc_byte);

        append_escape(&mut v, data);
        append_escape(&mut v, &[ZDLE, zcrc_byte]);
        v.extend_from_slice(&u16::to_le_bytes(crc));
        v
    }

    pub fn encode_subpacket_crc32(zcrc_byte: u8, data:&[u8]) -> Vec<u8>
    {
        let mut v = Vec::new();

        let mut crc = crc16::get_crc32(data);
        crc = !crc16::update_crc32(!crc, zcrc_byte);

        append_escape(&mut v, data);
        v.extend_from_slice(&[ZDLE, zcrc_byte]);
        append_escape(&mut v, &u32::to_le_bytes(crc));
        v
    }
}

const ESC_0X10:u8 = 0x10 ^ 0x40;
const ESC_0X90:u8 = 0x90 ^ 0x40;
const ESC_0X11:u8 = 0x11 ^ 0x40;
const ESC_0X91:u8 = 0x91 ^ 0x40;
const ESC_0X13:u8 = 0x13 ^ 0x40;
const ESC_0X93:u8 = 0x93 ^ 0x40;
const ESC_0X0D:u8 = 0x0D ^ 0x40;
const ESC_0X8D:u8 = 0x8D ^ 0x40;

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

impl Protocol for Zmodem  {

    fn get_current_state(&self) -> Option<super::TransferState> {
        self.transfer_state.clone()
    }

    fn update<T: crate::com::Com>(&mut self, com: &mut T) -> std::io::Result<()> {
        if self.sz.is_active() {
            self.sz.update(com)?;
        }
        Ok(())
    }

    fn initiate_send<T: crate::com::Com>(&mut self, com: &mut T, files: Vec<super::FileDescriptor>) -> std::io::Result<()> {
        self.sz.send(com, files)
    }

    fn initiate_recv<T: crate::com::Com>(&mut self, _com: &mut T) -> std::io::Result<()> {
        todo!()
    }

    fn get_received_files(&mut self) -> Vec<super::FileDescriptor> {
        todo!()
    }
}



#[cfg(test)]
mod tests {
    use std::vec;

    use crate::protocol::{Zmodem, ZCRCE};

    #[test]
    fn test_encode_subpckg_crc32() {
        let pck = Zmodem::encode_subpacket_crc32(ZCRCE, b"a\n");
        assert_eq!(vec![0x61, 0x0a, 0x18, 0x68, 0xe5, 0x79, 0xd2, 0x0f], pck);
    } 
}