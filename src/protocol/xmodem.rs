use std::io;

use crate::com::Com;

use super::{xymodem_core::XYmodem, Protocol, TransferState, FileDescriptor, FileTransferState};

pub struct Xmodem {
    core: XYmodem,
    transfer_state: Option<TransferState>,
    files: Vec<Vec<u8>>
}

impl Xmodem {
    pub fn new() -> Self {
        Self {
            core: XYmodem::new(),
            transfer_state: None,
            files: Vec::new()
        }
    }
}

impl Protocol for Xmodem
{
    fn get_current_state(&self) -> Option<TransferState>
    {
        self.transfer_state.clone()
    }

    fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        if let Some(state) = &self.transfer_state {
            self.core.update(com)?;
            if let super::xymodem_core::XYState::None = self.core.xy_state  {
                if state.recieve_state.is_some() {
                    let data = self.core.get_data()?;
                    self.files.push(data);
                }
                self.transfer_state = None;
            }
        }
        Ok(())
    }

    fn initiate_send<T: Com>(&mut self, com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        if files.len() != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Only 1 file can be send with x-modem."))
        }
        self.core.block_length = super::xymodem_core::DEFAULT_BLOCK_LENGTH;
        self.core.send(com, files)?;

        let mut state = TransferState::new();
        state.send_state = Some(FileTransferState::new());
        self.transfer_state = Some(state);
        Ok(())
    }
    
    fn initiate_recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.core.recv(com)?;
        self.core.files.push(FileDescriptor::new());
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
        let mut send = Xmodem::new();
        let mut recv = Xmodem::new();

        let data = vec![1u8, 2, 5, 10];
        let mut com = TestChannel::new();
        send.initiate_send(&mut com.sender, vec![FileDescriptor::create_test("foo.bar".to_string(), data.clone())]).expect("error.");
        recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while send.transfer_state.is_some() || recv.transfer_state.is_some()  {
            i += 1;
            if i > 10 { break; }
            send.update(&mut com.sender).expect("error.");
            recv.update(&mut com.receiver).expect("error.");
        }

        let rdata = recv.get_received_files();
        assert_eq!(1, rdata.len());
        let sdata = &rdata[0].get_data().unwrap();
        assert_eq!(&data, sdata);
    }

    #[test]
    fn test_longer_file() {
        for test_len in [128, 255, 256, 2048, 4097] {
            let mut send = Xmodem::new();
            let mut recv = Xmodem::new();

            let mut data = Vec::new();
            for i in 0..test_len {
                data.push(i as u8);
            }

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
            let sdata = &rdata[0].get_data().unwrap();
            assert_eq!(&data, sdata);
        }
    }
}
