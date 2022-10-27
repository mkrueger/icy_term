use std::io;

use crate::com::Com;
use super::{xymodem_core::XYmodem, Protocol, TransferState, FileDescriptor, FileTransferState, EXT_BLOCK_LENGTH};

pub struct Ymodem {
    core: XYmodem,
    transfer_state: Option<TransferState>
}

impl Ymodem {
    pub fn new() -> Self {
        let mut core = XYmodem::new();
        core.variant = super::XYModemVariant::YModem;
        Self {
            core,
            transfer_state: None
        }
    }
}

impl Protocol for Ymodem
{
    fn get_current_state(&self) -> Option<TransferState>
    {
        self.transfer_state.clone()
    }

    fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        if let Some(_) = &self.transfer_state {
            self.core.update(com)?;
            if let super::xymodem_core::XYState::None = self.core.xy_state  {
                self.transfer_state = None;
            }
        }
        Ok(())
    }

    fn initiate_send<T: Com>(&mut self, com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        self.core.block_length = EXT_BLOCK_LENGTH;
        self.core.send(com, files)?;

        let mut state = TransferState::new();
        state.send_state = Some(FileTransferState::new());
        self.transfer_state = Some(state);
        Ok(())
    }
    
    fn initiate_recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.core.block_length = EXT_BLOCK_LENGTH;
        self.core.recv(com)?;
        let mut state = TransferState::new();
        state.recieve_state = Some(FileTransferState::new());
        self.transfer_state = Some(state);

        Ok(())
    }

    fn get_received_files(&mut self) -> Vec<FileDescriptor>
    {
        let c = self.core.files.clone();
        self.core.files = Vec::new();
        c
    }
}

#[cfg(test)]
mod tests {
    use crate::{protocol::*, com::{TestChannel}};

    #[test]
    fn test_simple() {
        let mut send = Ymodem::new();
        let mut recv = Ymodem::new();

        let data = vec![1u8, 2, 5, 10];
        let mut com = TestChannel::new();
        send.initiate_send(&mut com.sender, vec![FileDescriptor::create_test("foo.bar".to_string(), data.clone())]).expect("error.");
        recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while send.transfer_state.is_some() || recv.transfer_state.is_some()  {
            i += 1;
            if i > 100 { break; }
            send.update(&mut com.sender).expect("error.");
            recv.update(&mut com.receiver).expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        assert_eq!(&data, &rdata[0].get_data().unwrap());
    }

    #[test]
    fn test_batch() {
        let mut send = Ymodem::new();
        let mut recv = Ymodem::new();

        let data1 = vec![1u8, 2, 5, 10];
        let data2 = vec![1u8, 42, 18, 19];
        let mut com = TestChannel::new();
        send.initiate_send(&mut com.sender, vec![
            FileDescriptor::create_test("foo.bar".to_string(), data1.clone()),
            FileDescriptor::create_test("baz".to_string(), data2.clone())]).expect("error.");

        recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while send.transfer_state.is_some() || recv.transfer_state.is_some()  {
            i += 1;
            if i > 100 { break; }
            send.update(&mut com.sender).expect("error.");
            recv.update(&mut com.receiver).expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(2, rdata.len());

        assert_eq!(&data1, &rdata[0].get_data().unwrap());
        assert_eq!(data1.len(), rdata[0].size);
       
        assert_eq!(&data2, &rdata[1].get_data().unwrap());
        assert_eq!(data2.len(), rdata[1].size);
    }
}
