use std::{time::Duration, io::{self}};
use std::{collections::HashMap};
use std:: { rc::Rc, cell::RefCell};

use super::Com;

pub struct TestCom {
    name: String,
    write_buf: Rc<RefCell<std::collections::VecDeque<u8>>>,
    read_buf: Rc<RefCell<std::collections::VecDeque<u8>>>,

    pub cmd_table: HashMap<u8, String>
}

pub fn indent_receiver()
{
    print!("\t\t\t\t\t\t");
}

impl Com for TestCom {
    fn get_name(&self) -> &'static str
    {
        "Test_Com"
    }

    fn read_char(&mut self, _timeout: Duration) -> io::Result<u8>
    {
        if self.name == "receiver" {
            indent_receiver();
        }

        if let Some(b) = self.read_buf.borrow_mut().pop_front() {
            println!("{} reads char {}/0x{:0X}({})", self.name, b, b, char::from_u32(b as u32).unwrap());
            return Ok(b);
        }
        panic!("should not happen!");
    }
    
    fn read_char_nonblocking(&mut self) -> io::Result<u8>
    {
        if self.name == "receiver" {
            indent_receiver();
        }

        if let Some(b) = self.read_buf.borrow_mut().pop_front() {
            println!("{} reads char {}({})", self.name, b, char::from_u32(b as u32).unwrap());
            return Ok(b);
        }
        panic!("should not happen!");
    }

    fn read_exact(&mut self, _duration: Duration, bytes: usize) -> io::Result<Vec<u8>>
    {
        if self.name == "receiver" {
            indent_receiver();
        }

        let b = self.read_buf.borrow_mut().drain(0..bytes).collect();
        println!("{} reads {:?}", self.name, b);
        Ok(b)
    }
    
    fn is_data_available(&mut self) -> io::Result<bool>
    {
        Ok(self.read_buf.borrow().len() > 0)
    }

    fn disconnect(&mut self) -> io::Result<()>
    {
        // nothing
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<()>
    {
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
        self.write_buf.borrow_mut().extend(buf.iter());
        Ok(())
    }
}


#[cfg(test)]
pub struct TestChannel {
    pub sender: Box<dyn Com>,
    pub receiver: Box<dyn Com>
}

#[cfg(test)]
impl TestChannel {
    pub fn new() -> Self {
        let b1 = Rc::new(RefCell::new(std::collections::VecDeque::new()));
        let b2 = Rc::new(RefCell::new(std::collections::VecDeque::new()));
        Self { 
            sender: Box::new(TestCom { 
                name: "sender".to_string(),
                read_buf:b1.clone(),
                write_buf:b2.clone(),
                cmd_table: HashMap::new()
            }), 
            receiver: Box::new(TestCom {
                name: "receiver".to_string(),
                read_buf:b2,
                write_buf:b1,
                cmd_table: HashMap::new()
            })
        }
    }
}

mod tests {
    use std::time::Duration;
    use crate::com::test_com::TestChannel;

    #[test]
    fn test_simple() {
        let mut test = TestChannel::new();
        let t = b"Hello World";
        test.sender.write(t).expect("error.");
        assert_eq!(t.to_vec(), test.receiver.read_exact(Duration::from_secs(1), t.len()).unwrap());
    }
}