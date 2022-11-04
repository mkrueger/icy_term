use std::{time::Duration, io::{self, ErrorKind}, net::SocketAddr};
#[cfg(test)]
use std::{collections::HashMap};

use telnet::Telnet;

pub trait Com
{
    fn get_name(&self) -> &'static str;

    fn read_char(&mut self, duration: Duration) -> io::Result<u8>;
    fn read_char_nonblocking(&mut self) -> io::Result<u8>;
    fn read_exact(&mut self, duration: Duration, bytes: usize) -> io::Result<Vec<u8>>;
    
    fn is_data_available(&mut self) -> io::Result<bool>;

    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;

    fn disconnect(&mut self);

}

pub struct TelnetCom
{
    telnet: Option<Telnet>,
    buf: std::collections::VecDeque<u8>
}

impl TelnetCom 
{
    pub fn connect(addr: &SocketAddr, _duration: Duration) -> io::Result<Self> {
        let t = Telnet::connect_timeout(addr, 256, Duration::from_secs(5))?;
        Ok(Self { 
            telnet: Some(t),
            buf: std::collections::VecDeque::new()
        })
    }

    fn fill_buffer(&mut self) -> io::Result<()>
    {
        if self.buf.len() > 0 { return Ok(()); }
        if let Some(t) = self.telnet.as_mut() {
            if let telnet::Event::Data(buffer) = t.read_nonblocking()? {
                if self.buf.try_reserve(buffer.len()).is_err() {
                    return Err(io::Error::new(ErrorKind::OutOfMemory, "out of memory"));
                }
                self.buf.extend(buffer.iter());
            }
            Ok(())
        } else {
            Err(io::Error::new(ErrorKind::OutOfMemory, "Connection error"))
        }
    }

    fn fill_buffer_wait(&mut self, timeout: Duration) -> io::Result<()>
    {
        if let Some(t) = self.telnet.as_mut() {
            if let telnet::Event::Data(buffer) = &t.read_timeout(timeout)? {
                if self.buf.try_reserve(buffer.len()).is_err() {
                    return Err(io::Error::new(ErrorKind::OutOfMemory, "out of memory"));
                }
                self.buf.extend(buffer.iter());
            }
            Ok(())
        } else {
            Err(io::Error::new(ErrorKind::OutOfMemory, "Connection error"))
        }
    }
}

impl Com for TelnetCom {
    fn get_name(&self) -> &'static str
    {
        "Telnet"
    }

    fn read_char(&mut self, timeout: Duration) -> io::Result<u8>
    {
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        self.fill_buffer_wait(timeout)?;
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        return Err(io::Error::new(ErrorKind::TimedOut, "timed out"));
    }
    
    fn read_char_nonblocking(&mut self) -> io::Result<u8>
    {
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        return Err(io::Error::new(ErrorKind::TimedOut, "no data avaliable"));
    }

    fn read_exact(&mut self, duration: Duration, bytes: usize) -> io::Result<Vec<u8>>
    {
        while self.buf.len() < bytes {
            self.fill_buffer_wait(duration)?;
        }
        Ok(self.buf.drain(0..bytes).collect())
    }
    
    fn is_data_available(&mut self) -> io::Result<bool>
    {
        self.fill_buffer()?; 
        Ok(self.buf.len() > 0)
    }

    fn disconnect(&mut self)
    {
        self.telnet = None;
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        if let Some(t) = &mut self.telnet {
            return t.write(buf);
        }
        return Err(io::Error::new(ErrorKind::NotConnected, "not connected"));
    }
}

#[cfg(test)]
use std:: { rc::Rc, cell::RefCell};

#[cfg(test)]
pub struct TestCom {
    name: String,
    write_buf: Rc<RefCell<std::collections::VecDeque<u8>>>,
    read_buf: Rc<RefCell<std::collections::VecDeque<u8>>>,

    pub cmd_table: HashMap<u8, String>
}

#[cfg(test)]
pub fn indent_receiver()
{
    print!("\t\t\t\t\t\t");
}

#[cfg(test)]
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

    fn disconnect(&mut self)
    {
        // nothing
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
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
        Ok(buf.len())
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

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use crate::com::{TestChannel};

    #[test]
    fn test_simple() {
        let mut test = TestChannel::new();
        let t = b"Hello World";
        test.sender.write(t).expect("error.");
        assert_eq!(t.to_vec(), test.receiver.read_exact(Duration::from_secs(1), t.len()).unwrap());
    }
}
