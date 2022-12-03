use async_trait::async_trait;

use crate::{com::{ Com, ComResult }};
use std::{ sync::{Arc, Mutex}};

mod constants;
mod ry;
mod sy;
mod tests;
mod error;

use self::{constants::{DEFAULT_BLOCK_LENGTH, EXT_BLOCK_LENGTH, CAN}, error::TransmissionError};

use super::{FileDescriptor, TransferState};
#[derive(Debug, Clone, Copy)]
pub enum Checksum {
    Default,
    CRC16,
}

#[derive(Debug, Clone, Copy)]
pub enum XYModemVariant {
    XModem,
    XModem1k,
    XModem1kG,
    YModem,
    YModemG,
}

/// specification: http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt
pub struct XYmodem {
    config: XYModemConfiguration,

    ry: Option<ry::Ry>,
    sy: Option<sy::Sy>,
}

impl XYmodem {
    pub fn new(variant: XYModemVariant) -> Self {
        XYmodem {
            config: XYModemConfiguration::new(variant),
            ry: None,
            sy: None,
        }
    }
}

#[async_trait]
impl super::Protocol for XYmodem {
    async fn update(&mut self, com: &mut Box<dyn Com>, transfer_state: Arc<Mutex<TransferState>>) -> ComResult<bool> {
        if let Some(ry) = &mut self.ry {
            ry.update(com, &transfer_state).await?;
            transfer_state.lock().unwrap().is_finished = ry.is_finished();
            if ry.is_finished() {
                return Ok(false);
            }
        } else if let Some(sy) = &mut self.sy {
            sy.update(com, &transfer_state).await?;
            transfer_state.lock().unwrap().is_finished = sy.is_finished();
             if sy.is_finished() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn initiate_send(
        &mut self,
        _com: &mut Box<dyn Com>,
        files: Vec<FileDescriptor>,
        transfer_state: Arc<Mutex<TransferState>>
    ) -> ComResult<()> {
        if !self.config.is_ymodem() && files.len() != 1 {
            return Err(Box::new(TransmissionError::XModem1File));
        }

        let mut sy = sy::Sy::new(self.config);
        // read data for x-modem transfer
        if !self.config.is_ymodem() {
            sy.data = files[0].get_data()?;
        }

        sy.send(files)?;
        self.sy = Some(sy);
        transfer_state.lock().unwrap().protocol_name = self.config.get_protocol_name().to_string();
        Ok(())
    }

    async fn initiate_recv(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: Arc<Mutex<TransferState>>
    ) -> ComResult<()> {
        let mut ry = ry::Ry::new(self.config);
        ry.recv(com).await?;
        self.ry = Some(ry);

        transfer_state.lock().unwrap().protocol_name = self.config.get_protocol_name().to_string();

        // Add ghost file with no name when receiving with x-modem because this protocol
        // doesn't transfer any file information. User needs to set a file name after download.
        if !self.config.is_ymodem() {
            self.ry.as_mut().unwrap().files.push(FileDescriptor::new());
        }

        Ok(())
    }

    fn get_received_files(&mut self) -> Vec<FileDescriptor> {
        if let Some(ry) = &mut self.ry {
            let c = ry.files.clone();
            ry.files = Vec::new();
            c
        } else {
            Vec::new()
        }
    }

    async fn cancel(&mut self, com: &mut Box<dyn Com>) -> ComResult<()> {
        cancel(com).await
    }
}

async fn cancel(com: &mut Box<dyn Com>) -> ComResult<()> {
    com.send(&[CAN, CAN, CAN, CAN, CAN, CAN]).await?;
    Ok(())
}

fn get_checksum(block: &[u8]) -> u8 {
    block.iter().fold(0, |x, &y| x.wrapping_add(y))
}

#[derive(Clone, Copy)]
pub struct XYModemConfiguration {
    pub variant: XYModemVariant,
    pub block_length: usize,
    pub checksum_mode: Checksum,
}

impl XYModemConfiguration {
    fn new(variant: XYModemVariant) -> Self {
        let (block_length, checksum_mode) = match variant {
            XYModemVariant::XModem => (DEFAULT_BLOCK_LENGTH, Checksum::Default),
            XYModemVariant::XModem1k
            | XYModemVariant::XModem1kG
            | XYModemVariant::YModem
            | XYModemVariant::YModemG => (EXT_BLOCK_LENGTH, Checksum::CRC16),
        };

        Self {
            variant,
            block_length,
            checksum_mode,
        }
    }

    fn get_protocol_name(&self) -> &str {
        match self.variant {
            XYModemVariant::XModem => "Xmodem",
            XYModemVariant::XModem1k => "Xmodem 1k",
            XYModemVariant::XModem1kG => "Xmodem 1k-G",
            XYModemVariant::YModem => "Ymodem",
            XYModemVariant::YModemG => "Ymodem-G",
        }
    }

    fn get_check_and_size(&self) -> String {
        let checksum = if let Checksum::Default = self.checksum_mode {
            "Checksum"
        } else {
            "Crc"
        };
        let block = if self.block_length == DEFAULT_BLOCK_LENGTH {
            "128"
        } else {
            "1k"
        };
        format!("{}/{}", checksum, block)
    }

    fn is_ymodem(&self) -> bool {
        match self.variant {
            XYModemVariant::YModem | XYModemVariant::YModemG => true,
            _ => false,
        }
    }

    fn is_streaming(&self) -> bool {
        match self.variant {
            XYModemVariant::XModem1kG | XYModemVariant::YModemG => true,
            _ => false,
        }
    }

    fn use_crc(&self) -> bool {
        match self.checksum_mode {
            Checksum::CRC16 => true,
            Checksum::Default => false,
        }
    }
}
