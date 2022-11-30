use icy_engine::get_crc16;
use std::{
    io::{self, ErrorKind},
    time::Duration,
};

use super::{
    constants::{CAN, DEFAULT_BLOCK_LENGTH},
    get_checksum, Checksum, XYModemConfiguration,
};
use crate::{
    com::Com,
    protocol::{
        str_from_null_terminated_utf8_unchecked,
        xymodem::constants::{ACK, CPMEOF, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
        FileDescriptor, TransferState,
    }, ui::main_window::Connection,
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

    _send_timeout: Duration,
    recv_timeout: Duration,
    _ack_timeout: Duration,

    pub files: Vec<FileDescriptor>,
    data: Vec<u8>,

    errors: usize,
    recv_state: RecvState,
}

impl Ry {
    pub fn new(configuration: XYModemConfiguration) -> Self {
        Ry {
            configuration,
            _send_timeout: Duration::from_secs(10),
            recv_timeout: Duration::from_secs(10),
            _ack_timeout: Duration::from_secs(3),

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

    pub fn update(&mut self, com: &mut Connection, state: &mut TransferState) -> io::Result<()> {
        if let Some(transfer_state) = &mut state.recieve_state {
            if self.files.len() > 0 {
                let cur_file = self.files.len() - 1;
                let f = &self.files[cur_file];
                transfer_state.file_name = f.file_name.clone();
                transfer_state.file_size = f.size;
            }
            transfer_state.bytes_transfered = self.bytes_send;
            transfer_state.errors = self.errors;
            transfer_state.check_size = self.configuration.get_check_and_size();
            transfer_state.update_bps();

            // println!("\t\t\t\t\t\t{:?}", self.recv_state);
            match self.recv_state {
                RecvState::None => {}

                RecvState::StartReceive(retries) => {
                    if com.is_data_available()? {
                        state.current_state = "Start receiving...";

                        let start = com.read_char(self.recv_timeout)?;
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
                                self.await_data(com)?;
                            } else if retries == 4 {
                                com.send(vec![NAK]);
                            } else {
                                self.cancel(com)?;
                                return Err(io::Error::new(
                                    ErrorKind::ConnectionAborted,
                                    "too many retries starting the communication",
                                ));
                            }
                            self.errors += 1;
                            self.recv_state = RecvState::StartReceive(retries + 1);
                        }
                    }
                }

                RecvState::ReadYModemHeader(retries) => {
                    let len = 128; // constant header length

                    if com.is_data_available()? {
                        state.current_state = "Get header...";
                        let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode
                        {
                            2
                        } else {
                            1
                        };

                        let block = com.read_exact(self.recv_timeout, 2 + len + chksum_size)?;

                        if block[0] != block[1] ^ 0xFF {
                            com.send(vec![NAK]);
                            self.errors += 1;
                            self.recv_state = RecvState::StartReceive(retries + 1);
                            return Ok(());
                        }
                        let block = &block[2..];
                        if !self.check_crc(block) {
                            //println!("NAK CRC FAIL");
                            self.errors += 1;
                            com.send(vec![NAK]);
                            self.recv_state = RecvState::ReadYModemHeader(retries + 1);
                            return Ok(());
                        }
                        if block[0] == 0 {
                            // END transfer
                            //println!("END TRANSFER");
                            com.send(vec![ACK]);
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
                        transfer_state.write(format!("Receiving file '{}'…", &fd.file_name));
                        self.files.push(fd);
                        if self.configuration.is_ymodem() {
                            com.send(vec![ACK, b'C']);
                        } else {
                            com.send(vec![ACK]);
                        }
                        self.recv_state = RecvState::ReadBlockStart(0, 0);
                    }
                }

                RecvState::ReadBlockStart(step, retries) => {
                    if com.is_data_available()? {
                        if step == 0 {
                            let start = com.read_char(self.recv_timeout)?;
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
                                let mut fd = self.files.get_mut(cur_file).unwrap();
                                fd.data = Some(self.data.clone());
                                self.data = Vec::new();

                                if self.configuration.is_ymodem() {
                                    com.send(vec![NAK]);
                                    self.recv_state = RecvState::ReadBlockStart(1, 0);
                                } else {
                                    com.send(vec![ACK]);
                                    self.recv_state = RecvState::None;
                                }
                            } else {
                                if retries < 5 {
                                    com.send(vec![NAK]);
                                } else {
                                    self.cancel(com)?;
                                    return Err(io::Error::new(
                                        ErrorKind::ConnectionAborted,
                                        "too many retries",
                                    ));
                                }
                                self.errors += 1;
                                self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                            }
                        } else if step == 1 {
                            let eot = com.read_char(self.recv_timeout)?;
                            if eot != EOT {
                                self.recv_state = RecvState::None;
                                return Ok(());
                            }
                            if self.configuration.is_ymodem() {
                                com.send(vec![ACK, b'C']);
                            } else {
                                com.send(vec![ACK]);
                            }
                            self.recv_state = RecvState::StartReceive(retries);
                        }
                    }
                }

                RecvState::ReadBlock(len, retries) => {
                    if com.is_data_available()? {
                        state.current_state = "Receiving data...";
                        let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode
                        {
                            2
                        } else {
                            1
                        };
                        let block = com.read_exact(self.recv_timeout, 2 + len + chksum_size)?;
                        if block[0] != block[1] ^ 0xFF {
                            com.send(vec![NAK]);

                            self.errors += 1;
                            let start = com.read_char(self.recv_timeout)?;
                            if start == SOH {
                                self.recv_state =
                                    RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                            } else if start == STX {
                                self.recv_state =
                                    RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                            } else {
                                self.cancel(com)?;
                                return Err(io::Error::new(
                                    ErrorKind::ConnectionAborted,
                                    "too many retries",
                                ));
                            }

                            self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                            return Ok(());
                        }
                        let block = &block[2..];
                        if !self.check_crc(&block) {
                            //println!("\t\t\t\t\t\trecv crc mismatch");
                            self.errors += 1;
                            com.send(vec![NAK]);
                            self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                            return Ok(());
                        }
                        self.data.extend_from_slice(&block[0..len]);
                        if !self.configuration.is_streaming() {
                            com.send(vec![ACK]);
                        }
                        self.recv_state = RecvState::ReadBlockStart(0, 0);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn cancel(&self, com: &mut Connection) -> io::Result<()> {
        com.send(vec![CAN, CAN]);
        Ok(())
    }

    pub fn recv(&mut self, com: &mut Connection) -> io::Result<()> {
        self.await_data(com)?;
        self.data = Vec::new();
        self.recv_state = RecvState::StartReceive(0);
        Ok(())
    }

    fn await_data(&mut self, com: &mut Connection) -> io::Result<usize> {
        if self.configuration.is_streaming() {
            com.send(vec![b'G']);
        } else if self.configuration.use_crc() {
            com.send(vec![b'C']);
        } else {
            com.send(vec![NAK]);
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
