use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use eframe::epaint::mutex::Mutex;

use crate::address_mod::Address;

use super::{Com, TermComResult};

pub struct TestCom {
    name: String,
    write_buf: Arc<Mutex<std::collections::VecDeque<u8>>>,
    read_buf: Arc<Mutex<std::collections::VecDeque<u8>>>,
    pub cmd_table: HashMap<u8, String>,
}

pub fn indent_receiver() {
    print!("\t\t\t\t\t\t");
}

impl Com for TestCom {
    fn get_name(&self) -> &'static str {
        "Test_Com"
    }

    fn set_terminal_type(&mut self, _terminal: crate::address_mod::Terminal) {}

    fn connect(&mut self, _addr: &Address, _timeout: Duration) -> TermComResult<bool> {
        Ok(true)
    }

    fn read_data(&mut self) -> TermComResult<Vec<u8>> {
        if self.name == "receiver" {
            indent_receiver();
        }
        let result: Vec<u8> = self.read_buf.lock().drain(0..).collect();

        if result.len() == 1 {
            if let Some(cmd) = self.cmd_table.get(&result[0]) {
                println!("{} {}({} 0x{})", self.name, cmd, result[0], result[0]);
            } else {
                println!("{} reads {} 0x{:X}", self.name, result[0], result[0]);
            }
        } else {
            println!("{} reads {:?} #{}", self.name, result, result.len());
        }

        Ok(result)
    }

    fn send(&mut self, buf: &[u8]) -> TermComResult<usize> {
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
        self.write_buf.lock().extend(buf.iter());
        Ok(buf.len())
    }

    fn read_u8(&mut self) -> TermComResult<u8> {
        if self.name == "receiver" {
            indent_receiver();
        } /*
          while self.read_buf.lock().is_empty() {
              tokio::time::sleep(Duration::from_millis(10)).await;
          }*/

        if let Some(b) = self.read_buf.lock().pop_front() {
            if let Some(cmd) = self.cmd_table.get(&b) {
                println!("{} {}({} 0x{})", self.name, cmd, b, b);
            } else {
                println!("{} reads {} 0x{:X}", self.name, b, b);
            }
            Ok(b)
        } else {
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No data to read",
            )))
        }
    }

    fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>> {
        let result: Vec<u8> = self.read_buf.lock().drain(0..len).collect();
        Ok(result)
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        // nothing
        Ok(())
    }
}

#[cfg(test)]
pub struct TestChannel {
    pub sender: Box<dyn Com>,
    pub receiver: Box<dyn Com>,
}

#[cfg(test)]
impl TestChannel {
    pub fn new() -> Self {
        let b1 = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        let b2 = Arc::new(Mutex::new(std::collections::VecDeque::new()));
        Self {
            sender: Box::new(TestCom {
                name: "sender".to_string(),
                read_buf: b1.clone(),
                write_buf: b2.clone(),
                cmd_table: HashMap::new(),
            }),
            receiver: Box::new(TestCom {
                name: "receiver".to_string(),
                read_buf: b2,
                write_buf: b1,
                cmd_table: HashMap::new(),
            }),
        }
    }
}

mod communication_tests {
    use crate::com::TestChannel;
    #[test]
    fn test_simple() {
        let mut test = TestChannel::new();
        let t = b"Hello World";
        let _ = test.sender.send(t);
        assert_eq!(t.to_vec(), test.receiver.read_data().unwrap());
    }

    #[test]
    fn test_transfer_byte() {
        let mut test = TestChannel::new();
        let _ = test.sender.send(&[42]);
        assert_eq!(42, test.receiver.read_u8().unwrap());
    }
}
