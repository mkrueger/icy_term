use crate::address::Address;

use super::Com;
#[allow(dead_code)]
use async_trait::async_trait;
use std::{io::ErrorKind, thread, time::Duration, collections::VecDeque};
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
};

pub struct SSHCom {
    tcp_stream: Option<TcpStream>,
    buf: std::collections::VecDeque<u8>,
}

impl SSHCom {
    pub fn new() -> Self {
        Self { tcp_stream: None, buf: VecDeque::new() }
    }

    fn fill_buffer(&mut self) -> io::Result<()> {
        let mut buf = [0; 1024 * 256];
        loop {
            match self.tcp_stream.as_mut().unwrap().try_read(&mut buf) {
                Ok(size) => {
                    self.buf.extend(&buf[0..size]);
                    return Ok(());
                }
                Err(ref e) => {
                    if e.kind() == io::ErrorKind::WouldBlock {
                        break;
                    }
                    return Err(io::Error::new(
                        ErrorKind::ConnectionAborted,
                        format!("Telnet error: {}", e),
                    ));
                }
            };
        }
        Ok(())
    }

    fn fill_buffer_wait(&mut self, _timeout: Duration) -> io::Result<()> {
        self.fill_buffer()?;
        while self.buf.len() == 0 {
            self.fill_buffer()?;
            thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

#[async_trait]
impl Com for SSHCom {
    fn get_name(&self) -> &'static str {
        "Raw"
    }
    async fn connect(&mut self, addr: &Address, timeout: Duration) -> Result<bool, String> {
        let r = tokio::time::timeout(timeout, TcpStream::connect(&addr.address)).await;
        match r {
            Ok(tcp_stream) => match tcp_stream {
                Ok(stream) => {
                    self.tcp_stream = Some(stream);
                    Ok(true)
                }
                Err(err) => Err(format!("{}", err)),
            },
            Err(err) => Err(format!("{}", err)),
        }
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

    async fn read_data(&mut self) -> io::Result<Vec<u8>> {
        self.fill_buffer()?;
        let r = self.buf.make_contiguous().to_vec();
        self.buf.clear();
        Ok(r)
    }

    fn disconnect(&mut self) -> io::Result<()> {
        Ok(())
    }
   
    async fn write<'a>(&mut self, buf: &'a [u8]) -> io::Result<usize> {
        Ok(self.tcp_stream.as_mut().unwrap().write(&buf).await?)
    }
}
