use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;

use crate::address_mod::Address;

use super::{Com, TermComResult};

#[derive(Debug)]
pub struct TestCom {
    _name: String,
    //  write_buf: Rc<RefCell<std::collections::VecDeque<u8>>>,
    // read_buf: Rc<RefCell<std::collections::VecDeque<u8>>>,
    pub cmd_table: HashMap<u8, String>,
}

pub fn _indent_receiver() {
    print!("\t\t\t\t\t\t");
}
#[async_trait]
impl Com for TestCom {
    fn get_name(&self) -> &'static str {
        "Test_Com"
    }

    async fn connect(&mut self, _addr: &Address, _timeout: Duration) -> TermComResult<bool> {
        Ok(true)
    }

    async fn read_data(&mut self) -> TermComResult<Vec<u8>> {
        todo!();
    }

    async fn send<'a>(&mut self, _buf: &'a [u8]) -> TermComResult<usize> {
        todo!();
    }

    async fn read_u8(&mut self) -> TermComResult<u8> {
        todo!();
    }

    async fn read_exact(&mut self, _len: usize) -> TermComResult<Vec<u8>> {
        todo!();
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        // nothing
        Ok(())
    }
    /*
    fn write(&mut self, buf: &[u8]) -> TerminalResult<usize> {
        if self.name == "receiver" {
            indent_receiver();
        }
        if buf.len() == 1 {
            if let Some(cmd) = self.cmd_table.get(&buf[0]) {
                println!("{} {}({} 0x{})", self.name, cmd, buf[0], buf[0]);
            } else {
                println!("{} writes {} 0x{:X}", self.name, buf[0], buf[0]);
            }
        } else {
            println!("{} writes {:?} #{}", self.name, buf, buf.len());
        }
        // self.write_buf.borrow_mut().extend(buf.iter());
        Ok(buf.len())
    }*/
}
/*
#[cfg(test)]
pub struct TestChannel {
    pub sender: Box<dyn Com>,
    pub receiver: Box<dyn Com>,
}

#[cfg(test)]
impl TestChannel {
    pub fn new() -> Self {
        //let b1 = Rc::new(RefCell::new(std::collections::VecDeque::new()));
        //let b2 = Rc::new(RefCell::new(std::collections::VecDeque::new()));
        Self {
            sender: Box::new(TestCom {
                _name: "sender".to_string(),
                //  read_buf:b1.clone(),
                //  write_buf:b2.clone(),
                cmd_table: HashMap::new(),
            }),
            receiver: Box::new(TestCom {
                _name: "receiver".to_string(),
                //   read_buf:b2,
                //    write_buf:b1,
                cmd_table: HashMap::new(),
            }),
        }
    }
}*/
/*
mod tests {
    use crate::com::test_com::TestChannel;
    use std::time::Duration;

    #[test]
    fn test_simple() {
        let mut test = TestChannel::new();
        let t = b"Hello World";
        test.sender.write(t);
        assert_eq!(
            t.to_vec(),
            test.receiver
                .read_exact(Duration::from_secs(1), t.len())
                .unwrap()
        );
    }
}*/
