use std::{time::{Duration, SystemTime}, io::{self, ErrorKind}, cmp::min};
use crate::{protocol::{FileDescriptor, TransferState, FileTransferState, xymodem::constants::{SOH, STX, EXT_BLOCK_LENGTH, EOT, CPMEOF, NAK, ACK}}, com::Com};
use super::{Checksum, get_checksum, XYModemVariant, constants::{CAN, DEFAULT_BLOCK_LENGTH}};


#[derive(Debug)]
pub enum SendState {
    None,
    InitiateSend,
    SendYModemHeader(usize),
    AckSendYmodemHeader(usize),
    SendData(usize, usize),
    AckSendData(usize, usize),
    YModemEndHeader(u8)
}

pub struct Sy {
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

    block_number: u8,
    errors: usize,
    send_state: SendState,

    last_header_send: SystemTime,

}

impl Sy {
    pub fn new() -> Self {
        Self {
            variant: XYModemVariant::XModem,
            block_length: DEFAULT_BLOCK_LENGTH,
            checksum_mode: Checksum::CRC16,
            streaming_mode: false,
            _send_timeout: Duration::from_secs(10),
            recv_timeout: Duration::from_secs(10),
            _ack_timeout: Duration::from_secs(3),
            last_header_send: SystemTime::UNIX_EPOCH,

            send_state: SendState::None,
            files: Vec::new(),
            data: Vec::new(),
            errors: 0,
            bytes_send: 0,
            block_number: 0,
            cur_file: 0
        }
    }

    pub fn is_finished(&self) -> bool { 
        if let SendState::None = self.send_state { true } else { false }
    }

    pub fn update<T: Com>(&mut self, com: &mut T, state: &mut TransferState) -> io::Result<()>
    {
        let mut transfer_state = FileTransferState::new();

        if self.cur_file < self.files.len() {
            let mut fd = FileDescriptor::new();
            let f = &self.files[self.cur_file];
            fd.file_name = f.file_name.clone();
            fd.size = f.size;
            transfer_state.file = Some(fd);
        }
        transfer_state.bytes_transfered = self.bytes_send;
        transfer_state.errors = self.errors;
        transfer_state.engine_state = format!("{:?}", self.send_state);
        state.send_state = Some(transfer_state);
        println!("send state: {:?} {:?}", self.send_state, self.variant);

        match self.send_state {
            SendState::None => Ok(()),

            SendState::InitiateSend => {
                state.current_state = "Initiate send...";
                if com.is_data_available()? {
                    self.get_mode(com)?;
                    if let XYModemVariant::YModem = self.variant {
                        self.last_header_send = SystemTime::UNIX_EPOCH;
                        self.send_state = SendState::SendYModemHeader(0);
                    } else {
                        self.send_state = SendState::SendData(0, 0);
                    }
                }
                Ok(())
            }
            SendState::SendYModemHeader(retries) => {
                let now = SystemTime::now();
                if now.duration_since(self.last_header_send).unwrap().as_millis() > 3000 {
                    self.last_header_send = now;
                    self.block_number = 0;
                    self.send_ymodem_header(com)?;
                    self.send_state = SendState::AckSendYmodemHeader(retries);
                }
                Ok(())
            },
            SendState::AckSendYmodemHeader(retries) => {
                if com.is_data_available()? {
                    let ack = com.read_char(self.recv_timeout)?;
                    if ack == NAK {
                        state.current_state = "Encountered error";
                        self.errors += 1;
                        if retries > 5 {
                            self.send_state = SendState::None;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries sending ymodem header")); 
                        }
                        self.last_header_send = SystemTime::UNIX_EPOCH;
                        self.send_state = SendState::SendYModemHeader(retries + 1);
                        return Ok(());
                    }
                    if ack == ACK {
                        state.current_state = "Header accepted.";
                        let _ = com.read_char(self.recv_timeout)?;
                        // SKIP - not needed to check that
                        self.send_state = SendState::SendData(0, 0);
                    }
                }
                Ok(())
            },
            SendState::SendData(cur_offset, retries) => {
                state.current_state = "Send data...";
                self.send_state = if !self.send_data_block(com, cur_offset)? {
                    SendState::None
                } else {
                    SendState::AckSendData(cur_offset, retries)
                };
                Ok(())
            }
            SendState::AckSendData(cur_offset, retries) => {
                if com.is_data_available()? {
                    let ack = com.read_char(self.recv_timeout)?;
                    if ack == CAN {
                        // need 2 CAN
                        let can2 = com.read_char(self.recv_timeout)?;
                        if can2 == CAN {
                            self.send_state = SendState::None;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "Connection was canceled.")); 
                        }
                    }

                    if ack != ACK {
                        self.errors += 1;

                        // fall back to short block length after too many errors 
                        if retries > 3 && self.block_length == EXT_BLOCK_LENGTH {
                            self.block_length = DEFAULT_BLOCK_LENGTH;
                            self.send_state = SendState::SendData(cur_offset, retries + 2);
                            return Ok(());
                        }

                        if retries > 5 {
                            self.send_state = SendState::None;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries sending ymodem header")); 
                        }
                        self.send_state = SendState::SendData(cur_offset, retries + 1);
                        return Ok(());
                    }
                    self.bytes_send = cur_offset + self.block_length;
                    self.send_state = SendState::SendData(self.bytes_send, 0);
                }

                if self.bytes_send >= self.files[self.cur_file].size {
                    com.write(&[EOT])?;
                    if let XYModemVariant::YModem = self.variant {
                        self.send_state = SendState::YModemEndHeader(0);
                    } else {
                        self.send_state = SendState::None;
                    }
                }
                Ok(())
            },
            SendState::YModemEndHeader(step)=> {
                match step {
                    0 => {
                        if com.is_data_available()? {
                            if com.read_char(self.recv_timeout)? == NAK {
                                com.write(&[EOT])?;
                                self.send_state = SendState::YModemEndHeader(1);
                                return Ok(());
                            }
                        }
                        self.cancel(com)?;
                        self.send_state = SendState::None;
                        return Ok(());
                    },
                    1 => {
                        if com.is_data_available()? {
                            if com.read_char(self.recv_timeout)? == ACK {
                                self.send_state = SendState::YModemEndHeader(2);
                                return Ok(());
                            }
                        }
                        self.cancel(com)?;
                        self.send_state = SendState::None;
                        return Ok(());
                    },
                    2 => {
                        if com.is_data_available()? {
                            if com.read_char(self.recv_timeout)? == b'C' {
                                self.last_header_send = SystemTime::UNIX_EPOCH;
                                self.send_state = SendState::SendYModemHeader(0);
                                self.cur_file += 1;
                                return Ok(());
                            }
                        }
                        self.cancel(com)?;
                        self.send_state = SendState::None;
                        return Ok(());
                    },
                    _ => { 
                        self.send_state = SendState::None;
                        return Ok(());
                    }
                }
            },
        }
    }
    
    pub fn get_mode<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        let ch = com.read_char(self.recv_timeout)?;
        match ch {
            NAK => {
                self.checksum_mode = Checksum::Default;
                return Ok(());
            },
            b'C' => {
                self.checksum_mode = Checksum::CRC16;
                return Ok(());
            },
            b'G' => {
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

    fn send_block<T: Com>(&mut self, com: &mut T, data: &[u8], pad_byte: u8) -> io::Result<()>
    {
        let block_len = if data.len() <= DEFAULT_BLOCK_LENGTH  { SOH } else { STX };

        let mut block = Vec::new();
        block.push(block_len);
        block.push(self.block_number);
        block.push(!self.block_number);
        block.extend_from_slice(data);
        block.resize((if block_len == SOH { DEFAULT_BLOCK_LENGTH } else { EXT_BLOCK_LENGTH }) + 3, pad_byte);

        match self.checksum_mode {
            Checksum::Default => {
                let chk_sum = get_checksum(&block[3..]);
                block.push(chk_sum);
            },
            Checksum::CRC16 => {
                let crc = crate::crc::get_crc16(&block[3..]);
                block.extend_from_slice(&u16::to_be_bytes(crc));
            },
        }
        com.write(&block)?;
        self.block_number += 1;
        Ok(())
    }

    fn send_ymodem_header<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        if self.cur_file < self.files.len() {
            // restart from 0
            let mut block = Vec::new();
            let fd = &self.files[self.cur_file];
            let name = fd.file_name.as_bytes();
            block.extend_from_slice(&name);
            block.push(0);
            block.extend_from_slice(format!("{}", fd.size).as_bytes());
            self.data = fd.get_data()?;
            self.send_block(com, &block, 0)?;
            Ok(())
        } else {
            self.end_ymodem(com)?;
            Ok(())
        }
    }

    fn send_data_block<T: Com>(&mut self, com: &mut T, offset: usize) -> io::Result<bool>
    {
        let data_len = self.files[self.cur_file].size;
        if offset >= data_len {
            return Ok(false);
        }
        let mut block_end = min(offset + self.block_length, data_len);
        
        if block_end - offset < EXT_BLOCK_LENGTH - 2 * DEFAULT_BLOCK_LENGTH {
            self.block_length = DEFAULT_BLOCK_LENGTH;
            block_end = min(offset + self.block_length, data_len);
        }
        self.send_block(com, &self.files[self.cur_file].data.as_ref().unwrap()[offset..block_end].to_vec(), CPMEOF)?;
        Ok(true)
    }

    pub fn cancel<T: Com>(&self, com: &mut T)-> io::Result<()> {
        com.write(&[CAN, CAN])?;
        Ok(())
    }

    pub fn send<T: Com>(&mut self, _com: &mut T, variant: XYModemVariant, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        match variant {
            XYModemVariant::XModem => self.block_length = DEFAULT_BLOCK_LENGTH,
            XYModemVariant::_XModem1k => self.block_length = EXT_BLOCK_LENGTH,
            XYModemVariant::YModem => self.block_length = EXT_BLOCK_LENGTH,
            XYModemVariant::_YModemG => self.block_length = EXT_BLOCK_LENGTH,
        }
        self.variant = variant;
        self.send_state = SendState::InitiateSend;
        self.files = files;
        self.cur_file = 0;
        self.bytes_send = 0;

        Ok(())
    }

    pub fn end_ymodem<T: Com>(&mut self, com: &mut T)-> io::Result<()> {
        self.send_block(com, &[0], 0)?;
        // TODO: Check ACK? Or skip?
        Ok(())
    }
}
    