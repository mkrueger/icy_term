use std::{time::Duration, cmp::min, io::{self, ErrorKind}, ffi::CStr};
use crate::com::Com;

use super::FileDescriptor;
const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;

const STX: u8 = 0x02;
const CPMEOF: u8 = 0x1A;

pub const DEFAULT_BLOCK_LENGTH: usize = 128;
pub const EXT_BLOCK_LENGTH: usize = 1024;

#[derive(Debug)]
pub enum Checksum {
    Default,
    CRC16,
}

#[derive(Debug)]
pub enum XYState {
    None,
    InitiateSend,
    SendYModemHeader(usize),
    AckSendYmodemHeader(usize),
    SendData(u8, usize, usize),
    AckSendData(u8, usize, usize),
    YModemEndHeader(u8),

    StartReceive(usize),
    ReadYModemHeader(usize),
    ReadBlock(usize, usize),
    ReadBlockStart(u8, usize)
}

pub enum XYModemVariant {
    XModem,
    YModem
}

/// specification: http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt
pub struct XYmodem {
    pub block_length: usize,
    pub bytes_send: usize,

    pub variant: XYModemVariant,
    checksum_mode: Checksum,
    streaming_mode: bool,

    _send_timeout: Duration,
    recv_timeout: Duration,
    _ack_timeout: Duration,

    pub files: Vec<FileDescriptor>,
    cur_file: usize,
    data: Vec<u8>,

    errors: usize,
    pub xy_state: XYState,
    is_sender: bool
}

impl XYmodem {
    pub fn new() -> Self {
        XYmodem {
            is_sender: false,
            variant: XYModemVariant::XModem,
            block_length: DEFAULT_BLOCK_LENGTH,
            checksum_mode: Checksum::CRC16,
            streaming_mode: false,
            _send_timeout: Duration::from_secs(10),
            recv_timeout: Duration::from_secs(10),
            _ack_timeout: Duration::from_secs(3),

            xy_state: XYState::None,
            files: Vec::new(),
            data: Vec::new(),
            errors: 0,
            bytes_send: 0,
            cur_file: 0
        }
    }

    pub fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        println!("{} state: {:?}", if self.is_sender { "sender" } else {"receiver"} , self.xy_state);
        match self.xy_state {
            XYState::None => Ok(()),

            XYState::InitiateSend => {
                if com.is_data_available()? {
                    self.get_mode(com)?;
                    if let XYModemVariant::YModem = self.variant {
                        self.xy_state = XYState::SendYModemHeader(0);
                    } else {
                        self.xy_state = XYState::SendData(0, 0, 0);
                    }
                }
                Ok(())
            }
            XYState::SendYModemHeader(retries) => {
                self.send_ymodem_header(com)?;
                self.xy_state = XYState::AckSendYmodemHeader(retries);
                Ok(())
            },
            XYState::AckSendYmodemHeader(retries) => {
                if com.is_data_available()? {
                    let ack = com.read_char(self.recv_timeout)?;
                    if ack == NAK {
                        self.errors += 1;
                        if retries > 5 {
                            self.xy_state = XYState::None;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries sending ymodem header")); 
                        }
                        self.xy_state = XYState::SendYModemHeader(retries + 1);
                        return Ok(());
                    }
                    if ack == ACK {
                        let _ = com.read_char(self.recv_timeout)?;
                        // SKIP - not needed to check that
                        self.xy_state = XYState::SendData(0, 0, 0);
                    }
                }
                Ok(())
            },
            XYState::SendData(block_num, cur_offset, retries) => {
                self.xy_state = if !self.send_data_block(com, block_num, cur_offset)? {
                    XYState::None
                } else {
                    XYState::AckSendData(block_num, cur_offset, retries)
                };
                Ok(())
            }
            XYState::AckSendData(block_num, cur_offset, retries) => {
                if com.is_data_available()? {
                    let ack = com.read_char(self.recv_timeout)?;
                    if ack == CAN {
                        // need 2 CAN
                        let can2 = com.read_char(self.recv_timeout)?;
                        if can2 == CAN {
                            self.xy_state = XYState::None;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "Connection was canceled.")); 
                        }
                    }

                    if ack != ACK {
                        self.errors += 1;

                        // fall back to short block length after too many errors 
                        if retries > 3 && self.block_length == EXT_BLOCK_LENGTH {
                            self.block_length = DEFAULT_BLOCK_LENGTH;
                            self.xy_state = XYState::SendData(block_num, cur_offset, retries + 2);
                            return Ok(());
                        }

                        if retries > 5 {
                            self.xy_state = XYState::None;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries sending ymodem header")); 
                        }
                        self.xy_state = XYState::SendData(block_num, cur_offset, retries + 1);
                        return Ok(());
                    }
                    self.bytes_send = cur_offset + self.block_length;
                    self.xy_state = XYState::SendData(block_num.wrapping_add(1), self.bytes_send, 0);
                }

                if self.bytes_send >= self.files[self.cur_file].size {
                    println!("write EOT {} {} ", self.bytes_send, self.data.len());
                    com.write(&[EOT])?;
                    if let XYModemVariant::YModem = self.variant {
                        self.xy_state = XYState::YModemEndHeader(0);
                    } else {
                        self.xy_state = XYState::None;
                    }
                }
                Ok(())
            },
            XYState::YModemEndHeader(step)=> {
                match step {
                    0 => {
                        if com.is_data_available()? {
                            if com.read_char(self.recv_timeout)? == NAK {
                                com.write(&[EOT])?;
                                self.xy_state = XYState::YModemEndHeader(1);
                                return Ok(());
                            }
                        }
                        self.cancel(com)?;
                        self.xy_state = XYState::None;
                        return Ok(());
                    },
                    1 => {
                        if com.is_data_available()? {
                            if com.read_char(self.recv_timeout)? == ACK {
                                self.xy_state = XYState::SendYModemHeader(0);
                                self.cur_file += 1;
                                return Ok(());
                            }
                        }
                        self.cancel(com)?;
                        self.xy_state = XYState::None;
                        return Ok(());
                    },
                    _ => { 
                        self.xy_state = XYState::None;
                        return Ok(());
                    }
                }
            },
            XYState::StartReceive(retries) => {
                if com.is_data_available()? {
                    let start = com.read_char(self.recv_timeout)?;
                    if start == SOH {
                        if let XYModemVariant::YModem = self.variant {
                            self.xy_state = XYState::ReadYModemHeader(0);
                        } else {
                            self.xy_state = XYState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                        }
                    } else if start == STX {
                        self.xy_state = XYState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                    } else {
                        if retries < 3 {
                            com.write(b"C")?;
                        } else if retries == 4  {
                            com.write(&[NAK])?;
                        } else {
                            self.cancel(com)?;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries starting the communication"));
                        }
                        self.errors += 1;
                        self.xy_state = XYState::StartReceive(retries + 1);
                    }
                }
                Ok(())
            },

            XYState::ReadYModemHeader(retries) => {
                let len = 128; // constant header length

                if com.is_data_available()? {
                    let block_num = com.read_char(self.recv_timeout)?;
                    let block_num_neg = com.read_char(self.recv_timeout)?;
        
                    if block_num != block_num_neg ^ 0xFF {
                        com.discard_buffer()?;
                        com.write(&[NAK])?;

                        self.errors += 1;
                        let start = com.read_char(self.recv_timeout)?;
                        if start == SOH {
                            self.xy_state = XYState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                        } else if start == STX {
                            self.xy_state = XYState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                        } else {
                            self.cancel(com)?;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries")); 
                        }
                        self.xy_state = XYState::ReadYModemHeader(retries + 1);
                        return Ok(());
                    }
                    let chksum_size = if let Checksum::CRC16 = self.checksum_mode { 2 } else { 1 };
                    let block = com.read_exact(self.recv_timeout, len + chksum_size)?;
                    if !self.check_crc(&block) {
                        self.errors += 1;
                        com.discard_buffer()?;
                        com.write(&[NAK])?;
                        self.xy_state = XYState::ReadYModemHeader(retries + 1);
                        return Ok(());
                    }
                    let mut fd =  FileDescriptor::new();
                    fd.file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
                    let num = str_from_null_terminated_utf8_unchecked(&block[(fd.file_name.len() + 1)..]).to_string();
                    if let Ok(file_size) = usize::from_str_radix(&num, 10) {
                        fd.size = file_size;
                    }
                    self.cur_file = self.files.len();
                    self.files.push(fd);
                    com.write(&[ACK, b'C'])?;
                    self.xy_state = XYState::ReadBlockStart(0, 0);
                }
                Ok(())
            },


            XYState::ReadBlockStart(step, retries) => {
                if com.is_data_available()? {
                    if step == 0 {
                        let start = com.read_char(self.recv_timeout)?;
                        if start == SOH {
                            self.xy_state = XYState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                        } else if start == STX {
                            self.xy_state = XYState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                        } else if start == STX {
                            self.xy_state = XYState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                        } else if start == EOT {

                            while self.data.ends_with(&[CPMEOF]) {
                                self.data.pop();
                            }
                            let mut p = self.files.get_mut(self.cur_file).unwrap();
                            p.data = Some(self.data.clone());
                            self.data = Vec::new();
                            self.cur_file += 1;
                            if let XYModemVariant::YModem = self.variant {
                                com.write(&[NAK])?;
                                self.xy_state = XYState::ReadBlockStart(1, 0);
                            } else {
                                com.write(&[ACK])?;
                                self.xy_state = XYState::None;
                            }
                        } else {
                            if retries < 5 {
                                com.write(&[NAK])?;
                            } else {
                                self.cancel(com)?;
                                return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries")); 
                            }
                            self.errors += 1;
                            self.xy_state = XYState::ReadBlockStart(0, retries + 1);
                        }
                    } else if step == 1 {
                        let eot = com.read_char(self.recv_timeout)?;
                        if eot != EOT {
                            self.xy_state = XYState::None;
                            return Ok(());
                        }
                        com.write(&[ACK, b'C'])?;
                        self.xy_state = XYState::StartReceive(retries);
                    }
                }
                Ok(())
            },

            XYState::ReadBlock(len, retries) => {
                if com.is_data_available()? {
                    let block_num = com.read_char(self.recv_timeout)?;
                    let block_num_neg = com.read_char(self.recv_timeout)?;
        
                    if block_num != block_num_neg ^ 0xFF {
                        com.discard_buffer()?;
                        com.write(&[NAK])?;

                        self.errors += 1;
                        let start = com.read_char(self.recv_timeout)?;
                        if start == SOH {
                            self.xy_state = XYState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                        } else if start == STX {
                            self.xy_state = XYState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                        } else {
                            self.cancel(com)?;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries")); 
                        }
                        
                        println!("invalid block number");
                        self.xy_state = XYState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                        return Ok(());
                    }
                    println!("checksum mode {:?}", self.checksum_mode);
                    let chksum_size = if let Checksum::CRC16 = self.checksum_mode { 2 } else { 1 };
                    let block = com.read_exact(self.recv_timeout, len + chksum_size)?;
                    if !self.check_crc(&block) {
                        println!("checksum failure!");
                        self.errors += 1;
                        com.discard_buffer()?;
                        com.write(&[NAK])?;
                        self.xy_state = XYState::ReadBlockStart(0, retries + 1);
                        return Ok(());
                    }
                    self.data.extend_from_slice(&block[0..len]);
                    com.write(&[ACK])?;
                    self.xy_state = XYState::ReadBlockStart(0, 0);
                }
                Ok(())
            }
        }
    }
    
    pub fn get_data(&mut self) -> io::Result<Vec<u8>>
    {
        let d = self.data.clone();
        self.data = Vec::new();
        Ok(d)
    }
    
    pub fn get_mode<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        let ch = com.read_char(self.recv_timeout)?;
        match ch {
            NAK => {
                println!("default chksm");
                self.checksum_mode = Checksum::Default;
                return Ok(());
            },
            b'C' => {
                println!("crc chksm");
                self.checksum_mode = Checksum::CRC16;
                return Ok(());
            },
            b'G' => {
                println!("got streaming mode!");
                self.streaming_mode = true;
                self.checksum_mode = Checksum::CRC16;
                return Ok(());
            },
            CAN => {
                return Err(io::Error::new(ErrorKind::InvalidData, "sending cancelled"));
            },
            _ => {
                return Err(io::Error::new(ErrorKind::InvalidData, format!("invalid x/y modem mode: {}", ch)));
            }
        }
    }

    fn send_ymodem_header<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        let mut block = Vec::new();
        block.push(SOH);
        block.push(0);
        block.push(0xFF);
        if self.cur_file < self.files.len() {
            let fd = &self.files[self.cur_file];
            let name = fd.file_name.as_bytes();
            block.extend_from_slice(&name);
            block.push(0);
            block.extend_from_slice(format!("{}", fd.size).as_bytes());

            block.resize(128 + 3, 0);

            let crc = crate::crc16::get_crc16(&block[3..]);
            block.extend_from_slice(&u16::to_be_bytes(crc));
            com.write(&block)?;
            self.data = fd.get_data()?;
            Ok(())
        } else {
            self.end_ymodem(com)?;
            Ok(())
        }
    }

    fn send_data_block<T: Com>(&mut self, com: &mut T, block_num: u8, offset: usize) -> io::Result<bool>
    {
        let data_len = self.files[self.cur_file].size;
        if offset >= data_len {
            return Ok(false);
        }
        let mut send_data = Vec::new();
        let mut block_end = min(offset + self.block_length, data_len);
        
        if block_end - offset < EXT_BLOCK_LENGTH - 2 * DEFAULT_BLOCK_LENGTH {
            self.block_length = DEFAULT_BLOCK_LENGTH;
            block_end = min(offset + self.block_length, data_len);
        }

        if self.block_length == EXT_BLOCK_LENGTH {
            send_data.push(STX);
        } else {
            send_data.push(SOH);
        }

        send_data.push(block_num);
        send_data.push(!block_num);
        send_data.extend_from_slice(&self.files[self.cur_file].data.as_ref().unwrap()[offset..block_end]);

        // fill last block with CPM_EOF
        send_data.resize(self.block_length as usize + 3, CPMEOF);

        match self.checksum_mode {
            Checksum::Default => {
                send_data.push(get_checksum(&send_data[3..]));
            },
            Checksum::CRC16 => {
                let crc = crate::crc16::get_crc16(&send_data[3..]);
                send_data.extend_from_slice(&u16::to_be_bytes(crc));
            },
        }
        com.write(&send_data)?;
        Ok(true)
    }

    pub fn cancel<T: Com>(&self, com: &mut T)-> io::Result<()> {
        com.write(&[CAN, CAN])?;
        Ok(())
    }

    pub fn send<T: Com>(&mut self, _com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        self.is_sender = true;
        self.xy_state = XYState::InitiateSend;
        self.files = files;
        self.cur_file = 0;
        self.bytes_send = 0;

        Ok(())
    }

    pub fn recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        com.write(b"C")?;
        self.is_sender = false;
        self.data = Vec::new();
        self.xy_state = XYState::StartReceive(0);
        Ok(())
    }

    pub fn end_ymodem<T: Com>(&self, _com: &mut T)-> io::Result<()> {
        let mut block = Vec::new();
        block.push(SOH);
        block.push(0);
        block.push(0xFF);
        block.resize(128 + 3, 0);
        let crc = crate::crc16::get_crc16(&block[3..]);
        block.push((crc >> 8) as u8);
        block.push(crc as u8);

        // TODO: Check ACK? Or skip?
        Ok(())
    }


    fn check_crc(&self, block: &[u8]) -> bool
    {
        if block.len() < 3 {
            return false;
        }
        match self.checksum_mode {
            Checksum::Default => {
                let chk = get_checksum(&block[..block.len() - 1]);
                block[block.len() - 1] != chk
            }
            Checksum::CRC16 => {
                let crc = crate::crc16::get_crc16(&block[..block.len() - 2]);
                block[block.len() - 2] != crc as u8 ||  block[block.len() - 1] != (crc >> 8) as u8
            }
        }
    }

}
    
fn get_checksum(block: &[u8]) -> u8 {
    block.iter().fold(0, |x, &y| x.wrapping_add(y))
}

fn str_from_null_terminated_utf8_unchecked(s: &[u8]) -> &str {
    if let Ok(s) = CStr::from_bytes_until_nul(s) {
        s.to_str().unwrap()
    } else {
        ""
    }
}