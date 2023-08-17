use icy_engine::get_crc16;
use std::io::{self, ErrorKind};

use super::{constants::DEFAULT_BLOCK_LENGTH, get_checksum, Checksum, XYModemConfiguration};
use crate::{
    com::{Com, TermComResult},
    protocol::{
        str_from_null_terminated_utf8_unchecked,
        xymodem::constants::{ACK, CPMEOF, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
        FileDescriptor, FileStorageHandler, TransferState,
    },
};

#[derive(Debug)]
pub enum RecvState {
    None,

    StartReceive(usize),
    ReadYModemHeader(usize),
    ReadBlock(usize, usize),
    ReadBlockStart(u8, usize),
}

/// specification: <http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt>
pub struct Ry {
    configuration: XYModemConfiguration,
    pub bytes_send: usize,

    pub files: Vec<FileDescriptor>,
    data: Vec<u8>,

    errors: usize,
    recv_state: RecvState,
}

impl Ry {
    pub fn new(configuration: XYModemConfiguration) -> Self {
        Ry {
            configuration,
            recv_state: RecvState::None,
            files: Vec::new(),
            data: Vec::new(),
            errors: 0,
            bytes_send: 0,
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.recv_state, RecvState::None)
    }

    pub fn update(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: &mut TransferState,
        storage_handler: &mut dyn FileStorageHandler,
    ) -> TermComResult<()> {
        let transfer_info = &mut transfer_state.recieve_state;
        if !self.files.is_empty() {
            let cur_file = self.files.len() - 1;
            let f = &self.files[cur_file];
            transfer_info.file_name = f.file_name.clone();
            transfer_info.file_size = f.size;
        }
        transfer_info.bytes_transfered = self.bytes_send;
        transfer_info.errors = self.errors;
        transfer_info.check_size = self.configuration.get_check_and_size();
        transfer_info.update_bps();

        match self.recv_state {
            RecvState::None => {}

            RecvState::StartReceive(retries) => {
                transfer_state.current_state = "Start receiving...";

                let start = com.read_u8()?;
                if start == SOH {
                    if self.configuration.is_ymodem() {
                        self.recv_state = RecvState::ReadYModemHeader(0);
                    } else {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    }
                } else if start == STX {
                    self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                } else {
                    if retries < 3 {
                        self.await_data(com)?;
                    } else if retries == 4 {
                        com.send(&[NAK])?;
                    } else {
                        self.cancel(com)?;
                        return Err(Box::new(io::Error::new(
                            ErrorKind::ConnectionAborted,
                            "too many retries starting the communication",
                        )));
                    }
                    self.errors += 1;
                    self.recv_state = RecvState::StartReceive(retries + 1);
                }
            }

            RecvState::ReadYModemHeader(retries) => {
                let len = 128; // constant header length

                transfer_state.current_state = "Get header...";
                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode {
                    2
                } else {
                    1
                };

                let block = com.read_exact(2 + len + chksum_size)?;

                if block[0] != block[1] ^ 0xFF {
                    com.send(&[NAK])?;
                    self.errors += 1;
                    self.recv_state = RecvState::StartReceive(retries + 1);
                    return Ok(());
                }
                let block = &block[2..];
                if !self.check_crc(block) {
                    //println!("NAK CRC FAIL");
                    self.errors += 1;
                    com.send(&[NAK])?;
                    self.recv_state = RecvState::ReadYModemHeader(retries + 1);
                    return Ok(());
                }
                if block[0] == 0 {
                    // END transfer
                    //println!("END TRANSFER");
                    com.send(&[ACK])?;
                    self.recv_state = RecvState::None;
                    return Ok(());
                }

                let mut fd = FileDescriptor::new();
                fd.file_name = str_from_null_terminated_utf8_unchecked(block);
                let num =
                    str_from_null_terminated_utf8_unchecked(&block[(fd.file_name.len() + 1)..])
                        .to_string();
                if let Ok(file_size) = num.parse::<usize>() {
                    fd.size = file_size;
                }
                self.files.push(fd);
                if self.configuration.is_ymodem() {
                    com.send(&[ACK, b'C'])?;
                } else {
                    com.send(&[ACK])?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }

            RecvState::ReadBlockStart(step, retries) => {
                if step == 0 {
                    let start = com.read_u8()?;
                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                    } else if start == EOT {
                        while self.data.ends_with(&[CPMEOF]) {
                            self.data.pop();
                        }
                        if self.files.is_empty() {
                            self.files.push(FileDescriptor::new());
                        }

                        let cur_file = self.files.len() - 1;
                        let fd = self.files.get_mut(cur_file).unwrap();
                        storage_handler.open_file(&fd.file_name, 0);
                        storage_handler.append(&self.data);
                        storage_handler.close();
                        self.data = Vec::new();

                        if self.configuration.is_ymodem() {
                            com.send(&[NAK])?;
                            self.recv_state = RecvState::ReadBlockStart(1, 0);
                        } else {
                            com.send(&[ACK])?;
                            self.recv_state = RecvState::None;
                        }
                    } else {
                        if retries < 5 {
                            com.send(&[NAK])?;
                        } else {
                            self.cancel(com)?;
                            return Err(Box::new(io::Error::new(
                                ErrorKind::ConnectionAborted,
                                "too many retries",
                            )));
                        }
                        self.errors += 1;
                        self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    }
                } else if step == 1 {
                    let eot = com.read_u8()?;
                    if eot != EOT {
                        self.recv_state = RecvState::None;
                        return Ok(());
                    }
                    if self.configuration.is_ymodem() {
                        com.send(&[ACK, b'C'])?;
                    } else {
                        com.send(&[ACK])?;
                    }
                    self.recv_state = RecvState::StartReceive(retries);
                }
            }

            RecvState::ReadBlock(len, retries) => {
                transfer_state.current_state = "Receiving data...";
                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode {
                    2
                } else {
                    1
                };
                let block = com.read_exact(2 + len + chksum_size)?;
                if block[0] != block[1] ^ 0xFF {
                    com.send(&[NAK])?;

                    self.errors += 1;
                    let start = com.read_u8()?;
                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                    } else {
                        self.cancel(com)?;
                        return Err(Box::new(io::Error::new(
                            ErrorKind::ConnectionAborted,
                            "too many retries",
                        )));
                    }

                    self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                    return Ok(());
                }
                let block = &block[2..];
                if !self.check_crc(block) {
                    //println!("\t\t\t\t\t\trecv crc mismatch");
                    self.errors += 1;
                    com.send(&[NAK])?;
                    self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    return Ok(());
                }
                self.data.extend_from_slice(&block[0..len]);
                if !self.configuration.is_streaming() {
                    com.send(&[ACK])?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }
        }
        Ok(())
    }

    pub fn cancel(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()> {
        self.recv_state = RecvState::None;
        super::cancel(com)
    }

    pub fn recv(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()> {
        self.await_data(com)?;
        self.data = Vec::new();
        self.recv_state = RecvState::StartReceive(0);
        Ok(())
    }

    fn await_data(&mut self, com: &mut Box<dyn Com>) -> TermComResult<usize> {
        if self.configuration.is_streaming() {
            com.send(&[b'G'])?;
        } else if self.configuration.use_crc() {
            com.send(&[b'C'])?;
        } else {
            com.send(&[NAK])?;
        }
        Ok(1)
    }

    fn check_crc(&self, block: &[u8]) -> bool {
        if block.len() < 3 {
            return false;
        }
        match self.configuration.checksum_mode {
            Checksum::Default => {
                let chk = get_checksum(&block[..block.len() - 1]);
                block[block.len() - 1] == chk
            }
            Checksum::CRC16 => {
                let check_crc = get_crc16(&block[..block.len() - 2]);
                let crc = u16::from_be_bytes(block[block.len() - 2..].try_into().unwrap());
                crc == check_crc
            }
        }
    }
}
