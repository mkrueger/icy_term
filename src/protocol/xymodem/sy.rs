use icy_engine::get_crc16;
use std::{
    cmp::min,
    time::{SystemTime}, sync::{Arc, Mutex},
};

use super::{
    constants::{CAN, DEFAULT_BLOCK_LENGTH},
    get_checksum, Checksum, XYModemConfiguration, XYModemVariant, error::TransmissionError,
};
use crate::{
    protocol::{
        xymodem::constants::{ACK, CPMEOF, EOT, EXT_BLOCK_LENGTH, NAK, SOH, STX},
        FileDescriptor, TransferState,
    }, com::{Com, ComResult}
};

#[derive(Debug)]
pub enum SendState {
    None,
    InitiateSend,
    SendYModemHeader(usize),
    AckSendYmodemHeader(usize),
    SendData(usize, usize),
    AckSendData(usize, usize),
    YModemEndHeader(u8),
}

pub struct Sy {
    pub bytes_send: usize,
    configuration: XYModemConfiguration,

    pub files: Vec<FileDescriptor>,
    cur_file: usize,

    block_number: u8,
    errors: usize,
    send_state: SendState,

    pub data: Vec<u8>,

    transfer_stopped: bool,
}

impl Sy {
    pub fn new(configuration: XYModemConfiguration) -> Self {
        Self {
            configuration,

            send_state: SendState::None,
            files: Vec::new(),
            data: Vec::new(),
            errors: 0,
            bytes_send: 0,
            block_number: 0,
            cur_file: 0,
            transfer_stopped: false,
        }
    }

    pub fn is_finished(&self) -> bool {
        if let SendState::None = self.send_state {
            true
        } else {
            false
        }
    }

    pub async fn update(&mut self, com: &mut Box<dyn Com>, state: &Arc<Mutex<TransferState>>) -> ComResult<()> {
        if let Ok(transfer_state) = &mut state.lock() {
            let transfer_info = &mut transfer_state.send_state;
            if self.cur_file < self.files.len() {
                let f = &self.files[self.cur_file];
                transfer_info.file_name = f.file_name.clone();
                transfer_info.file_size = f.size;
            }
            transfer_info.bytes_transfered = self.bytes_send;
            transfer_info.errors = self.errors;
            transfer_info.check_size = self.configuration.get_check_and_size();
            transfer_info.update_bps();
        }
        // println!("send state: {:?} {:?}", self.send_state, self.configuration.variant);

        match self.send_state {
            SendState::None => {}
            SendState::InitiateSend => {
                state.lock().unwrap().current_state = "Initiate send…";
                self.get_mode(com).await?;
                if self.configuration.is_ymodem() {
                    self.send_state = SendState::SendYModemHeader(0);
                } else {
                    self.send_state = SendState::SendData(0, 0);
                }
            }

            SendState::SendYModemHeader(retries) => {
                if retries > 3 {
                    state.lock().unwrap().current_state = "Too many retries...aborting";
                    self.cancel(com).await?;
                    return Ok(());
                }
                self.block_number = 0;
                //transfer_info.write("Send header...".to_string());
                self.send_ymodem_header(com).await?;
                self.send_state = SendState::AckSendYmodemHeader(retries);
            }
            
            SendState::AckSendYmodemHeader(retries) => {
                let now = SystemTime::now();
                let ack = self.read_command(com).await?;
                if ack == NAK {
                    state.lock().unwrap().current_state = "Encountered error";
                    self.errors += 1;
                    if retries > 5 {
                        self.send_state = SendState::None;
                        return Err(Box::new(TransmissionError::TooManyRetriesSendingHeader));
                    }
                    self.send_state = SendState::SendYModemHeader(retries + 1);
                    return Ok(());
                }
                if ack == ACK {
                    if self.transfer_stopped {
                        self.send_state = SendState::None;
                        return Ok(());
                    }
                    state.lock().unwrap().current_state = "Header accepted.";
                    self.data = self.files[self.cur_file].get_data()?;
                    let _ = self.read_command(com).await?;
                    // SKIP - not needed to check that
                    self.send_state = SendState::SendData(0, 0);
                }

               /*  if now
                    .duration_since(self.last_header_send)
                    .unwrap()
                    .as_millis()
                    > 3000
                {
                    self.send_state = SendState::SendYModemHeader(retries + 1);
                }*/
            }
            SendState::SendData(cur_offset, retries) => {
                state.lock().unwrap().current_state = "Send data...";
                if !self.send_data_block(com, cur_offset).await? {
                    self.send_state = SendState::None;
                } else {
                    if self.configuration.is_streaming() {
                        self.bytes_send = cur_offset + self.configuration.block_length;
                        self.send_state = SendState::SendData(self.bytes_send, 0);
                        self.check_eof(com).await?;
                    } else {
                        self.send_state = SendState::AckSendData(cur_offset, retries);
                    }
                };
            }
            SendState::AckSendData(cur_offset, retries) => {
                let ack = self.read_command(com).await?;
                if ack == CAN {
                    // need 2 CAN
                    let can2 = self.read_command(com).await?;
                    if can2 == CAN {
                        self.send_state = SendState::None;
                        //transfer_info.write("Got cancel ...".to_string());
                        return Err(Box::new(TransmissionError::Cancel));
                    }
                }

                if ack != ACK {
                    self.errors += 1;

                    // fall back to short block length after too many errors
                    if retries > 3 && self.configuration.block_length == EXT_BLOCK_LENGTH {
                        self.configuration.block_length = DEFAULT_BLOCK_LENGTH;
                        self.send_state = SendState::SendData(cur_offset, retries + 2);
                        return Ok(());
                    }

                    if retries > 5 {
                        self.eot(com).await?;
                        return Err(Box::new(TransmissionError::TooManyRetriesSendingHeader));
                    }
                    self.send_state = SendState::SendData(cur_offset, retries + 1);
                    return Ok(());
                }
                self.bytes_send = cur_offset + self.configuration.block_length;
                self.send_state = SendState::SendData(self.bytes_send, 0);
                self.check_eof(com).await?;
            }
            SendState::YModemEndHeader(step) => match step {
                0 => {
                    if self.read_command(com).await? == NAK {
                        com.send(&[EOT]).await?;
                        self.send_state = SendState::YModemEndHeader(1);
                        return Ok(());
                    }
                    self.cancel(com).await?;
                }
                1 => {
                    if self.read_command(com).await? == ACK {
                        self.send_state = SendState::YModemEndHeader(2);
                        return Ok(());
                    }
                    self.cancel(com).await?;
                }
                2 => {
                    if self.read_command(com).await? == b'C' {
                        self.send_state = SendState::SendYModemHeader(0);
                        self.cur_file += 1;
                        return Ok(());
                    }
                    self.cancel(com).await?;
                }
                _ => {
                    self.send_state = SendState::None;
                }
            },
            _  => {}
        }
        Ok(())
    }

    async fn check_eof(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        if self.bytes_send >= self.files[self.cur_file].size {
            self.eot(com).await?;
            if self.configuration.is_ymodem() {
                self.send_state = SendState::YModemEndHeader(0);
            } else {
                self.send_state = SendState::None;
            }
        }
        Ok(())
    }

    async fn read_command(&self, com: &mut Box<dyn Com>) -> ComResult<u8> {
        let ch = com.read_u8().await?;
        /* let cmd = match ch {
            b'C' => "[C]",
            EOT => "[EOT]",
            ACK => "[ACK]",
            NAK => "[NAK]",
            CAN => "[CAN]",
            _ => ""
        };

        if cmd.len() > 0 {
            "GOT CMD: {}", cmd);
        } else {
            println!("GOT CMD: #{} (0x{:X})", ch, ch);
        }*/

        Ok(ch)
    }

    async fn eot(&self, com: &mut Box<dyn Com>) -> ComResult<usize> {
        // println!("[EOT]");
        com.send(&[EOT]).await;
        Ok(1)
    }

    pub async fn get_mode(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        let ch = self.read_command(com).await?;
        match ch {
            NAK => {
                self.configuration.checksum_mode = Checksum::Default;
                return Ok(());
            }
            b'C' => {
                self.configuration.checksum_mode = Checksum::CRC16;
                return Ok(());
            }
            b'G' => {
                self.configuration = if self.configuration.is_ymodem() {
                    XYModemConfiguration::new(XYModemVariant::YModemG)
                } else {
                    XYModemConfiguration::new(XYModemVariant::XModem1kG)
                };
                return Ok(());
            }
            CAN => {
                return Err(Box::new(TransmissionError::Cancel));
            }
            _ => {
                return Err(Box::new(TransmissionError::InvalidMode(ch)));
            }
        }
    }

    async fn send_block(&mut self, com: &mut Box<dyn Com>, data: &[u8], pad_byte: u8) -> ComResult<()> {
        let block_len = if data.len() <= DEFAULT_BLOCK_LENGTH {
            SOH
        } else {
            STX
        };
        let mut block = Vec::new();
        block.push(block_len);
        block.push(self.block_number);
        block.push(!self.block_number);
        block.extend_from_slice(data);
        block.resize(
            (if block_len == SOH {
                DEFAULT_BLOCK_LENGTH
            } else {
                EXT_BLOCK_LENGTH
            }) + 3,
            pad_byte,
        );

        match self.configuration.checksum_mode {
            Checksum::Default => {
                let chk_sum = get_checksum(&block[3..]);
                block.push(chk_sum);
            }
            Checksum::CRC16 => {
                let crc = get_crc16(&block[3..]);
                block.extend_from_slice(&u16::to_be_bytes(crc));
            }
        }
        // println!("Send block {:X?}", block);
        com.send(&block).await?;
        self.block_number = self.block_number.wrapping_add(1);
        Ok(())
    }

    async fn send_ymodem_header(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        if self.cur_file < self.files.len() {
            // restart from 0
            let mut block = Vec::new();
            let fd = &self.files[self.cur_file];
            let name = fd.file_name.as_bytes();
            block.extend_from_slice(&name);
            block.push(0);
            block.extend_from_slice(format!("{}", fd.size).as_bytes());
            self.send_block(com, &block, 0).await?;
            Ok(())
        } else {
            self.end_ymodem(com).await?;
            Ok(())
        }
    }

    async fn send_data_block(&mut self, com: &mut Box<dyn Com>, offset: usize) -> ComResult<bool> {
        let data_len = self.data.len();
        if offset >= data_len {
            return Ok(false);
        }
        let mut block_end = min(offset + self.configuration.block_length, data_len);

        if block_end - offset < EXT_BLOCK_LENGTH - 2 * DEFAULT_BLOCK_LENGTH {
            self.configuration.block_length = DEFAULT_BLOCK_LENGTH;
            block_end = min(offset + self.configuration.block_length, data_len);
        }
        self.send_block(
            com,
            &Vec::from_iter(self.data[offset..block_end].iter().cloned()),
            CPMEOF,
        ).await?;
        Ok(true)
    }

    pub async fn cancel(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        self.send_state = SendState::None;
        super::cancel(com).await
    }

    pub fn send(&mut self, files: Vec<FileDescriptor>) -> ComResult<()> {
        self.send_state = SendState::InitiateSend;
        self.files = files;
        self.cur_file = 0;
        self.bytes_send = 0;

        Ok(())
    }

    pub async fn end_ymodem(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        self.send_block(com, &[0], 0).await?;
        self.transfer_stopped = true;
        Ok(())
    }
}
