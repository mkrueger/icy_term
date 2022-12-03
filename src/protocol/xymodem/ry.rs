use directories::UserDirs;
use icy_engine::get_crc16;
use std::{
    io::{self, ErrorKind}, sync::{Arc, Mutex}, fs::{self},
};

use super::{
    constants::{DEFAULT_BLOCK_LENGTH},
    get_checksum, Checksum, XYModemConfiguration,
};
use crate::{
    protocol::{
        str_from_null_terminated_utf8_unchecked,
        xymodem::constants::{ACK, CPMEOF, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
        FileDescriptor, TransferState,
    },  com::{Com, ComResult}
};

#[derive(Debug)]
pub enum RecvState {
    None,

    StartReceive(usize),
    ReadYModemHeader(usize),
    ReadBlock(usize, usize),
    ReadBlockStart(u8, usize),
}

/// specification: http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt
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
        if let RecvState::None = self.recv_state {
            true
        } else {
            false
        }
    }

    pub async fn update(&mut self, com: &mut Box<dyn Com>, state: &Arc<Mutex<TransferState>>) -> ComResult<()> {

        if let Ok(transfer_state) = &mut state.lock() {
            let transfer_info = &mut transfer_state.recieve_state;
            if self.files.len() > 0 {
                let cur_file = self.files.len() - 1;
                let f = &self.files[cur_file];
                transfer_info.file_name = f.file_name.clone();
                transfer_info.file_size = f.size;
            }
            transfer_info.bytes_transfered = self.bytes_send;
            transfer_info.errors = self.errors;
            transfer_info.check_size = self.configuration.get_check_and_size();
            transfer_info.update_bps();
        }

        // println!("\t\t\t\t\t\t{:?}", self.recv_state);
        match self.recv_state {
            RecvState::None => {}

            RecvState::StartReceive(retries) => {
                state.lock().unwrap().current_state = "Start receiving...";

                let start = com.read_u8().await?;
                // println!("{:02X} {}, {}", start, start, char::from_u32(start as u32).unwrap());
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
                        self.await_data(com).await?;
                    } else if retries == 4 {
                        com.send(&[NAK]).await?;
                    } else {
                        self.cancel(com).await?;
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

                state.lock().unwrap().current_state = "Get header...";
                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode
                {
                    2
                } else {
                    1
                };

                let block = com.read_exact(2 + len + chksum_size).await?;

                if block[0] != block[1] ^ 0xFF {
                    com.send(&[NAK]).await?;
                    self.errors += 1;
                    self.recv_state = RecvState::StartReceive(retries + 1);
                    return Ok(());
                }
                let block = &block[2..];
                if !self.check_crc(block) {
                    //println!("NAK CRC FAIL");
                    self.errors += 1;
                    com.send(&[NAK]).await?;
                    self.recv_state = RecvState::ReadYModemHeader(retries + 1);
                    return Ok(());
                }
                if block[0] == 0 {
                    // END transfer
                    //println!("END TRANSFER");
                    com.send(&[ACK]).await?;
                    self.recv_state = RecvState::None;
                    return Ok(());
                }

                let mut fd = FileDescriptor::new();
                fd.file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
                let num = str_from_null_terminated_utf8_unchecked(
                    &block[(fd.file_name.len() + 1)..],
                )
                .to_string();
                if let Ok(file_size) = usize::from_str_radix(&num, 10) {
                    fd.size = file_size;
                }
                self.files.push(fd);
                if self.configuration.is_ymodem() {
                    com.send(&[ACK, b'C']).await?;
                } else {
                    com.send(&[ACK]).await?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }

            RecvState::ReadBlockStart(step, retries) => {
                if step == 0 {
                    let start = com.read_u8().await?;
                    if start == SOH {
                        self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, 0);
                    } else if start == STX {
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, 0);
                    } else if start == EOT {
                        while self.data.ends_with(&[CPMEOF]) {
                            self.data.pop();
                        }
                        if self.files.len() == 0 {
                            self.files.push(FileDescriptor::new());
                        }

                        let cur_file = self.files.len() - 1;
                        let fd = self.files.get_mut(cur_file).unwrap();
                        if let Some(user_dirs) = UserDirs::new() {
                            let dir = user_dirs.download_dir().unwrap();

                            let mut file_name = dir.join(&fd.file_name);
                            let mut i = 1;
                            while file_name.exists() {
                                file_name = dir.join(&format!("{}.{}", fd.file_name, i));
                                i += 1;
                            }
                            fs::write(file_name, &self.data)?;
                        }
                        self.data = Vec::new();

                        if self.configuration.is_ymodem() {
                            com.send(&[NAK]).await?;
                            self.recv_state = RecvState::ReadBlockStart(1, 0);
                        } else {
                            com.send(&[ACK]).await?;
                            self.recv_state = RecvState::None;
                        }
                    } else {
                        if retries < 5 {
                            com.send(&[NAK]).await?;
                        } else {
                            self.cancel(com).await?;
                            return Err(Box::new(io::Error::new(
                                ErrorKind::ConnectionAborted,
                                "too many retries",
                            )));
                        }
                        self.errors += 1;
                        self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    }
                } else if step == 1 {
                    let eot = com.read_u8().await?;
                    if eot != EOT {
                        self.recv_state = RecvState::None;
                        return Ok(());
                    }
                    if self.configuration.is_ymodem() {
                        com.send(&[ACK, b'C']).await?;
                    } else {
                        com.send(&[ACK]).await?;
                    }
                    self.recv_state = RecvState::StartReceive(retries);
                }
            }

            RecvState::ReadBlock(len, retries) => {
                state.lock().unwrap().current_state = "Receiving data...";
                let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode
                {
                    2
                } else {
                    1
                };
                let block = com.read_exact(2 + len + chksum_size).await?;
                if block[0] != block[1] ^ 0xFF {
                    com.send(&[NAK]).await?;

                    self.errors += 1;
                    let start = com.read_u8().await?;
                    if start == SOH {
                        self.recv_state =
                            RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                    } else if start == STX {
                        self.recv_state =
                            RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                    } else {
                        self.cancel(com).await?;
                        return Err(Box::new(io::Error::new(
                            ErrorKind::ConnectionAborted,
                            "too many retries",
                        )));
                    }

                    self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                    return Ok(());
                }
                let block = &block[2..];
                if !self.check_crc(&block) {
                    //println!("\t\t\t\t\t\trecv crc mismatch");
                    self.errors += 1;
                    com.send(&[NAK]).await?;
                    self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                    return Ok(());
                }
                self.data.extend_from_slice(&block[0..len]);
                if !self.configuration.is_streaming() {
                    com.send(&[ACK]).await?;
                }
                self.recv_state = RecvState::ReadBlockStart(0, 0);
            }
        }
        Ok(())
    }

    pub async fn cancel(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        self.recv_state = RecvState::None;
        super::cancel(com).await
    }

    pub async fn recv(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        self.await_data(com).await?;
        self.data = Vec::new();
        self.recv_state = RecvState::StartReceive(0);
        Ok(())
    }

    async fn await_data(&mut self, com: &mut Box<dyn Com>) -> ComResult<usize> {
        if self.configuration.is_streaming() {
            com.send(&[b'G']).await?;
        } else if self.configuration.use_crc() {
            com.send(&[b'C']).await?;
        } else {
            com.send(&[NAK]).await?;
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
