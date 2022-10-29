use std::{time::Duration, io::{self, ErrorKind}};
use crate::{protocol::{FileDescriptor, TransferState, FileTransferState, xymodem::constants::{SOH, STX, EXT_BLOCK_LENGTH, EOT, CPMEOF, NAK, ACK}}, com::Com};
use super::{Checksum, get_checksum,  constants::{CAN, DEFAULT_BLOCK_LENGTH}, XYModemConfiguration};


#[derive(Debug)]
pub enum RecvState {
    None,

    StartReceive(usize),
    ReadYModemHeader(usize),
    ReadBlock(usize, usize),
    ReadBlockStart(u8, usize)
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
    recv_state: RecvState
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
            bytes_send: 0
        }
    }

    pub fn is_finished(&self) -> bool { 
        if let RecvState::None = self.recv_state { true } else { false }
    }

    pub fn update<T: Com>(&mut self, com: &mut T, state: &mut TransferState) -> io::Result<()>
    {
        let mut transfer_state = FileTransferState::new();
        if self.files.len() > 0 {
            let cur_file = self.files.len() - 1;
            let f = &self.files[cur_file];
            transfer_state.file_name = f.file_name.clone();
            transfer_state.file_size = f.size;
        }
        transfer_state.bytes_transfered = self.bytes_send;
        transfer_state.errors = self.errors;
        transfer_state.engine_state = format!("{:?}", self.recv_state);
        transfer_state.check_size = self.configuration.get_check_and_size();
        state.recieve_state = Some(transfer_state);

        // println!("\t\t\t\t\t\t{:?}", self.recv_state);
        match self.recv_state {
            RecvState::None => Ok(()),

            RecvState::StartReceive(retries) => {
                if com.is_data_available()? {
                    state.current_state = "Start receiving...";

                    let start = com.read_char(self.recv_timeout)?;
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
                        } else if retries == 4  {
                            com.write(&[NAK])?;
                        } else {
                            self.cancel(com)?;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries starting the communication"));
                        }
                        self.errors += 1;
                        self.recv_state = RecvState::StartReceive(retries + 1);
                    }
                }
                Ok(())
            },

            RecvState::ReadYModemHeader(retries) => {
                let len = 128; // constant header length

                if com.is_data_available()? {
                    state.current_state = "Get header...";

                    let block_num = com.read_char(self.recv_timeout)?;
                    let block_num_neg = com.read_char(self.recv_timeout)?;
        
                    if block_num != block_num_neg ^ 0xFF {
                        //println!("BLOCK ERROR");
                        com.discard_buffer()?;
                        com.write(&[NAK])?;

                        self.errors += 1;
                        let start = com.read_char(self.recv_timeout)?;
                        if start == SOH {
                            self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                        } else if start == STX {
                            self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                        } else {
                            self.cancel(com)?;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries")); 
                        }
                        self.recv_state = RecvState::ReadYModemHeader(retries + 1);
                        return Ok(());
                    }
                    let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                    let block = com.read_exact(self.recv_timeout, len + chksum_size)?;
                    if !self.check_crc(&block) {
                       // println!("NAK CRC FAIL");
                        self.errors += 1;
                        com.discard_buffer()?;
                        com.write(&[NAK])?;
                        self.recv_state = RecvState::ReadYModemHeader(retries + 1);
                        return Ok(());
                    }

                    if block[0] == 0 {
                        // END transfer
                        // println!("END TRANSFER");
                        com.write(&[ACK])?;
                        self.recv_state = RecvState::None;
                        return Ok(());
                    }

                    let mut fd =  FileDescriptor::new();
                    fd.file_name = str_from_null_terminated_utf8_unchecked(&block).to_string();
                    let num = str_from_null_terminated_utf8_unchecked(&block[(fd.file_name.len() + 1)..]).to_string();
                    if let Ok(file_size) = usize::from_str_radix(&num, 10) {
                        fd.size = file_size;
                    }
                    self.files.push(fd);
                    if self.configuration.is_ymodem() {
                        com.write(&[ACK, b'C'])?;
                    } else {
                        com.write(&[ACK])?;
                    }
                    self.recv_state = RecvState::ReadBlockStart(0, 0);
                }
                Ok(())
            },

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
                                com.write(&[NAK])?;
                                self.recv_state = RecvState::ReadBlockStart(1, 0);
                            } else {
                                com.write(&[ACK])?;
                                self.recv_state = RecvState::None;
                            }
                        } else {
                            if retries < 5 {
                                com.write(&[NAK])?;
                            } else {
                                self.cancel(com)?;
                                return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries")); 
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
                            com.write(&[ACK, b'C'])?;
                        } else {
                            com.write(&[ACK])?;
                        }
                        self.recv_state = RecvState::StartReceive(retries);
                    }
                }
                Ok(())
            },

            RecvState::ReadBlock(len, retries) => {
                if com.is_data_available()? {
                    state.current_state = "Receiving data...";
                    let block_num = com.read_char(self.recv_timeout)?;
                    let block_num_neg = com.read_char(self.recv_timeout)?;
        
                    if block_num != block_num_neg ^ 0xFF {
                        com.discard_buffer()?;
                        com.write(&[NAK])?;

                        self.errors += 1;
                        let start = com.read_char(self.recv_timeout)?;
                        if start == SOH {
                            self.recv_state = RecvState::ReadBlock(DEFAULT_BLOCK_LENGTH, retries + 1);
                        } else if start == STX {
                            self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                        } else {
                            self.cancel(com)?;
                            return Err(io::Error::new(ErrorKind::ConnectionAborted, "too many retries")); 
                        }
                        
                        self.recv_state = RecvState::ReadBlock(EXT_BLOCK_LENGTH, retries + 1);
                        return Ok(());
                    }
                    let chksum_size = if let Checksum::CRC16 = self.configuration.checksum_mode { 2 } else { 1 };
                    let block = com.read_exact(self.recv_timeout, len + chksum_size)?;
                    if !self.check_crc(&block) {
                        // println!("\t\t\t\t\t\trecv crc mismatch");
                        self.errors += 1;
                        com.discard_buffer()?;
                        com.write(&[NAK])?;
                        self.recv_state = RecvState::ReadBlockStart(0, retries + 1);
                        return Ok(());
                    }
                    self.data.extend_from_slice(&block[0..len]);
                    if !self.configuration.is_streaming() {
                        com.write(&[ACK])?;
                    }
                    self.recv_state = RecvState::ReadBlockStart(0, 0);
                }
                Ok(())
            }
        }
    }

    pub fn cancel<T: Com>(&self, com: &mut T)-> io::Result<()> {
        com.write(&[CAN, CAN])?;
        Ok(())
    }

    pub fn recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.await_data(com)?;
        self.data = Vec::new();
        self.recv_state = RecvState::StartReceive(0);
        Ok(())
    }

    fn await_data<T: Com>(&mut self, com: &mut T) -> io::Result<usize> {
        if self.configuration.is_streaming() {
            com.write(b"G")
        } else if self.configuration.use_crc() {
            com.write(b"C")
        } else {
            com.write(&[NAK])
        }
    }

    fn check_crc(&self, block: &[u8]) -> bool
    {
        if block.len() < 3 {
            return false;
        }
        match self.configuration.checksum_mode {
            Checksum::Default => {
                let chk = get_checksum(&block[..block.len() - 1]);
                block[block.len() - 1] == chk
            }
            Checksum::CRC16 => {
                let check_crc = crate::crc::get_crc16(&block[..block.len() - 2]);
                let crc = u16::from_be_bytes(block[block.len() - 2..].try_into().unwrap());
                crc == check_crc
            }
        }
    }
}

fn str_from_null_terminated_utf8_unchecked(s: &[u8]) -> &str {
    if let Ok(s) = std::ffi::CStr::from_bytes_until_nul(s) {
        s.to_str().unwrap()
    } else {
        ""
    }
}