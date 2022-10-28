use std::io;

use crate::com::Com;
use super::{xymodem::XYmodem, Protocol, TransferState, FileDescriptor, XYModemVariant};

pub struct Ymodem {
    core: XYmodem
}

impl Ymodem {
    pub fn new() -> Self {
        Self {
            core: XYmodem::new()
        }
    }
}

impl Protocol for Ymodem
{
    fn get_name(&self) -> &str
    {
        "Ymodem"
    }

    fn get_current_state(&self) -> Option<TransferState>
    {
        self.core.get_current_state()
    }
    fn is_active(&self) -> bool { self.core.is_active() }

    fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.core.update(com)
    }
    fn initiate_send<T: Com>(&mut self, com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>
    {
        self.core.send(com, XYModemVariant::YModem, files)
    }
    fn initiate_recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.core.recv(com, XYModemVariant::YModem)
    }
    fn get_received_files(&mut self) -> Vec<FileDescriptor>
    {
        self.core.get_received_files()
    }
    fn cancel<T: crate::com::Com>(&mut self, com: &mut T) -> io::Result<()>
    {
        self.core.cancel(com)
    }
}

#[cfg(test)]
mod tests {
    use crate::{protocol::*, com::{TestChannel, TestCom}};

    fn setup_xmodem_cmds(com: &mut TestCom) {
        com.cmd_table.insert(b'C', "C".to_string());
        com.cmd_table.insert(0x04, "EOT".to_string());
        com.cmd_table.insert(0x06, "ACK".to_string());
        com.cmd_table.insert(0x15, "NAK".to_string());
        com.cmd_table.insert(0x18, "CAN".to_string());
    }

    fn create_channel() -> TestChannel {
        let mut res = TestChannel::new();
        setup_xmodem_cmds(&mut res.sender);
        setup_xmodem_cmds(&mut res.receiver);
        res
    }

    #[test]
    fn test_simple() {
        let mut send = Ymodem::new();
        let mut recv = Ymodem::new();
        
        let data = vec![1u8, 2, 5, 10];
        let mut com = create_channel();

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
        assert_eq!(&data, &rdata[0].get_data().unwrap());
    }

  
   

    #[test]
    fn test_batch() {
        let mut send = Ymodem::new();
        let mut recv = Ymodem::new();

        let data1 = vec![1u8, 2, 5, 10];
        let data2 = vec![1u8, 42, 18, 19];
        let mut com = create_channel();
        send.initiate_send(&mut com.sender, vec![
            FileDescriptor::create_test("foo.bar".to_string(), data1.clone()),
            FileDescriptor::create_test("baz".to_string(), data2.clone())]).expect("error.");

        recv.initiate_recv(&mut com.receiver).expect("error.");
        let mut i = 0;
        while send.is_active() || recv.is_active()  {
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
