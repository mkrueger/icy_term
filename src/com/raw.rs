#[allow(dead_code)]
use std::{io::{ErrorKind, self, Read, Write}, time::Duration, net::{SocketAddr, TcpStream}, thread};
use super::Com;

pub struct RawCom
{
    tcp_stream: TcpStream,
    buf: std::collections::VecDeque<u8>
}

impl RawCom 
{
    pub fn connect(addr: &SocketAddr, timeout: Duration) -> io::Result<Self> {
        let tcp_stream = std::net::TcpStream::connect_timeout(addr, timeout)?;
        tcp_stream.set_nonblocking(true)?;

        Ok(Self { 
            tcp_stream,
            buf: std::collections::VecDeque::new()
        })
    }

    fn fill_buffer(&mut self) -> io::Result<()> {
        let mut buf = [0;1024 * 8];
        loop {
            match self.tcp_stream.read(&mut buf) {
                Ok(size) => {
                    self.buf.extend_from_slice(&buf[0..size]);
                    break;
                }
                Err(ref e) => {
                    if e.kind() == io::ErrorKind::WouldBlock {
                        break;
                    }
                    return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("{}", e)));
                }
            };
        }
        Ok(())
    }

    fn fill_buffer_wait(&mut self, _timeout: Duration) -> io::Result<()> {
        self.tcp_stream.set_nonblocking(false)?;
        self.fill_buffer()?;
        while self.buf.len() == 0 {
            self.fill_buffer()?;
            thread::sleep(Duration::from_millis(10));
        }
        self.tcp_stream.set_nonblocking(true)?;
        Ok(())
    }
}

impl Com for RawCom {
    fn get_name(&self) -> &'static str {
        "Raw"
    }

    fn read_char(&mut self, timeout: Duration) -> io::Result<u8> {
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        self.fill_buffer_wait(timeout)?;
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        return Err(io::Error::new(ErrorKind::TimedOut, "timed out"));
    }
    
    fn read_char_nonblocking(&mut self) -> io::Result<u8> {
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        return Err(io::Error::new(ErrorKind::TimedOut, "no data avaliable"));
    }

    fn read_exact(&mut self, duration: Duration, bytes: usize) -> io::Result<Vec<u8>> {
        while self.buf.len() < bytes {
            self.fill_buffer_wait(duration)?;
        }
        Ok(self.buf.drain(0..bytes).collect())
    }
    
    fn is_data_available(&mut self) -> io::Result<bool> {
        self.fill_buffer()?; 
        Ok(self.buf.len() > 0)
    }

    fn disconnect(&mut self) -> io::Result<()> {
        self.tcp_stream.shutdown(std::net::Shutdown::Both)
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        self.tcp_stream.write_all(&buf)
    }
}
