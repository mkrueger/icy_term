use std::io;
use crate::com::Com;
use super::{xymodem::XYmodem, Protocol, TransferState, FileDescriptor, XYModemVariant};

pub struct Xmodem {
    core: XYmodem
}

impl Xmodem {
    pub fn new() -> Self {
        Self {
            core: XYmodem::new()
        }
    }
}

impl Protocol for Xmodem
{
    fn get_name(&self) -> &str
    {
        "Xmodem"
    }
    
    fn get_current_state(&self) -> Option<TransferState> { self.core.get_current_state() }
    fn is_active(&self) -> bool { self.core.is_active() }

    fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.core.update(com)?;

        Ok(())
    }

    fn initiate_send<T: Com>(&mut self, com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        if files.len() != 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Only 1 file can be send with x-modem."))
        }
        self.core.send(com, XYModemVariant::XModem, files)?;
/* 
        let mut state = TransferState::new();
        state.send_state = Some(FileTransferState::new());
        self.transfer_state = Some(state);*/
        Ok(())
    }
    
    fn initiate_recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.core.recv(com, XYModemVariant::XModem)?;
        /* self.core.files.push(FileDescriptor::new());
        let mut state = TransferState::new();
        state.recieve_state = Some(FileTransferState::new());
        self.transfer_state = Some(state);*/
        Ok(())
    }

    fn get_received_files(&mut self) -> Vec<FileDescriptor> { self.core.get_received_files() }
    fn cancel<T: crate::com::Com>(&mut self, com: &mut T) -> io::Result<()> { self.core.cancel(com) }
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
        while send.is_active() || recv.is_active()  {
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
            while send.is_active() || recv.is_active()  {
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
