use std::{io::{self}};
use crate::com::Com;

mod sy;
mod ry;
mod constants;

use self::constants::CAN;

use super::{FileDescriptor, TransferState, FileTransferState};
#[derive(Debug)]
pub enum Checksum {
    Default,
    CRC16,
}

#[derive(Debug, Clone, Copy)]
pub enum XYModemVariant {
    XModem,
    _XModem1k,
    YModem,
    _YModemG
}

/// specification: http://pauillac.inria.fr/~doligez/zmodem/ymodem.txt
pub struct XYmodem {
    transfer_state: Option<TransferState>,

    ry: Option<ry::Ry>,
    sy: Option<sy::Sy>
}

impl XYmodem {
    pub fn new() -> Self {
        XYmodem {
            transfer_state: None,
            ry: None,
            sy: None
        }
    }

    pub fn get_current_state(&self) -> Option<TransferState>
    {
        self.transfer_state.clone()
    }
    
    pub fn is_active(&self) -> bool {
        self.transfer_state.is_some()
    }

    pub fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
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
    
    pub fn send<T: Com>(&mut self, com: &mut T, variant: XYModemVariant, files: Vec<FileDescriptor>) -> io::Result<()>
    {
       let mut sy = sy::Sy::new();
       sy.send(com, variant, files)?;
       self.sy = Some(sy);
       let mut state = TransferState::new();
       state.send_state = Some(FileTransferState::new());
       self.transfer_state = Some(state);

       Ok(())
    }

    pub fn recv<T: Com>(&mut self, com: &mut T, variant: XYModemVariant) -> io::Result<()>
    {
       let mut ry = ry::Ry::new();
       ry.recv(com, variant)?;
       self.ry = Some(ry);
        
       let mut state = TransferState::new();
       state.recieve_state = Some(FileTransferState::new());
       self.transfer_state = Some(state);

       // Add ghost file with no name when receiving with x-modem because this protocol
       // doesn't transfer any file information. User needs to set a file name after download.
       match variant {
           XYModemVariant::XModem | XYModemVariant::_XModem1k => {
               self.ry.as_mut().unwrap().files.push(FileDescriptor::new());
           }
           _ => {}
       }
    
       Ok(())
    }

    pub fn get_received_files(&mut self) -> Vec<FileDescriptor>
    {
        if let Some(ry) = &mut self.ry {
            let c = ry.files.clone();
            ry.files = Vec::new();
            c
        } else {
            Vec::new()
        }
    }

    pub fn cancel<T: crate::com::Com>(&mut self, com: &mut T) -> io::Result<()>
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

