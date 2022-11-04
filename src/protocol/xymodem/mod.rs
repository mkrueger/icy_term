use std::{io::{self}};
use crate::com::Com;

mod sy;
mod ry;
mod constants;
mod tests;

use self::constants::{CAN, DEFAULT_BLOCK_LENGTH, EXT_BLOCK_LENGTH};

use super::{FileDescriptor, TransferState, FileTransferState};
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
    YModemG
}

/// specification: http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt
pub struct XYmodem {
    transfer_state: Option<TransferState>,
    config: XYModemConfiguration,

    ry: Option<ry::Ry>,
    sy: Option<sy::Sy>
}

impl XYmodem {
    pub fn new(variant: XYModemVariant) -> Self {
        XYmodem {
            transfer_state: None,
            config: XYModemConfiguration::new(variant),
            ry: None,
            sy: None
        }
    }

}

impl super::Protocol for XYmodem {

    fn get_name(&self) -> &str {
        self.config.get_protocol_name()
    }

    fn get_current_state(&self) -> Option<&TransferState> {
       self.transfer_state.as_ref()
    }
    
    fn is_active(&self) -> bool {
        self.transfer_state.is_some()
    }

    fn update(&mut self, com: &mut Box<dyn Com>) -> io::Result<()> {
        if self.transfer_state.is_none() {
            return Ok(());
        }

        if let Some(ry) = &mut self.ry {
            ry.update(com, self.transfer_state.as_mut().unwrap())?;

            if ry.is_finished() {
                self.transfer_state = None;
            }
        } else if let Some(sy) = &mut self.sy {
            sy.update(com, self.transfer_state.as_mut().unwrap())?;
            if sy.is_finished() {
                self.transfer_state = None;
            }
        }
        Ok(())
    }
    
    fn initiate_send(&mut self, com: &mut Box<dyn Com>, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        if !self.config.is_ymodem() && files.len() != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Only 1 file can be send with x-modem."))
        }

       let mut sy = sy::Sy::new(self.config);
       // read data for x-modem transfer
       if !self.config.is_ymodem() {
           sy.data = files[0].get_data()?;
       }

       sy.send(com, files)?;
       self.sy = Some(sy);
       let mut state = TransferState::new();
       state.send_state = Some(FileTransferState::new());
       self.transfer_state = Some(state);
       
       Ok(())
    }

    fn initiate_recv(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>
    {
       let mut ry = ry::Ry::new(self.config);
       ry.recv(com)?;
       self.ry = Some(ry);
        
       let mut state = TransferState::new();
       state.recieve_state = Some(FileTransferState::new());
       self.transfer_state = Some(state);

       // Add ghost file with no name when receiving with x-modem because this protocol
       // doesn't transfer any file information. User needs to set a file name after download.
       if !self.config.is_ymodem() {
        self.ry.as_mut().unwrap().files.push(FileDescriptor::new());
    }
    
       Ok(())
    }

    fn get_received_files(&mut self) -> Vec<FileDescriptor>
    {
        if let Some(ry) = &mut self.ry {
            let c = ry.files.clone();
            ry.files = Vec::new();
            c
        } else {
            Vec::new()
        }
    }

    fn cancel(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>
    {
        self.transfer_state = None;
        com.write(&[CAN, CAN])?;
        com.write(&[CAN, CAN])?;
        com.write(&[CAN, CAN])?;
        Ok(())
    }
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
        let (block_length, checksum_mode) =
        match variant {
            XYModemVariant::XModem => (DEFAULT_BLOCK_LENGTH, Checksum::Default),
            XYModemVariant::XModem1k |
            XYModemVariant::XModem1kG |
            XYModemVariant::YModem |
            XYModemVariant::YModemG => (EXT_BLOCK_LENGTH, Checksum::CRC16),
        };

        Self {
            variant,
            block_length,
            checksum_mode
        }
    }

    fn get_protocol_name(&self) -> &str
    {
        match self.variant {
            XYModemVariant::XModem => "Xmodem",
            XYModemVariant::XModem1k => "Xmodem 1k",
            XYModemVariant::XModem1kG => "Xmodem 1k-G",
            XYModemVariant::YModem => "Ymodem",
            XYModemVariant::YModemG =>  "Ymodem-G",
        }
    }

    fn get_check_and_size(&self) -> String {
        let checksum = if let Checksum::Default = self.checksum_mode {  "Checksum" } else { "Crc"};
        let block = if self.block_length == DEFAULT_BLOCK_LENGTH { "128" } else { "1k" };
        format!("{}/{}", checksum, block)
    }


    fn is_ymodem(&self) -> bool {
        match self.variant {
            XYModemVariant::YModem | XYModemVariant::YModemG => true,
            _ => false
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