use super::Com;
#[allow(dead_code)]
use async_trait::async_trait;
use std::{io::ErrorKind, thread, time::Duration};
use tokio::{
    io::{self, AsyncReadExt},
    net::TcpStream,
};

pub struct RawCom {
    tcp_stream: TcpStream,
    buf: std::collections::VecDeque<u8>,
}

impl RawCom {
    fn fill_buffer(&mut self) -> io::Result<()> {
        let mut buf = [0; 1024 * 256];
        loop {
            match self.tcp_stream.try_read(&mut buf) {
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
impl Com for RawCom {
    fn get_name(&self) -> &'static str {
        "Raw"
    }
    async fn connect(&mut self, addr: String) -> Result<bool, String> {
        let r = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr)).await;
        match r {
            Ok(tcp_stream) => match tcp_stream {
                Ok(stream) => {
                    self.tcp_stream = stream;
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

    fn disconnect(&mut self) -> io::Result<()> {
        Ok(())
    }
   
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tcp_stream.try_write(&buf)
    }
}
