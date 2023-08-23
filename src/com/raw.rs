#![allow(dead_code)]

use super::{Com, OpenConnectionData, TermComResult};
use std::{
    io::{self, ErrorKind, Read, Write},
    net::TcpStream,
    time::Duration,
};

pub struct ComRawImpl {
    tcp_stream: TcpStream,
}

impl ComRawImpl {
    pub fn connect(connection_data: &OpenConnectionData) -> TermComResult<Self> {
        let tcp_stream = TcpStream::connect(&connection_data.address)?;
        tcp_stream.set_nonblocking(true)?;
        tcp_stream.set_read_timeout(Some(Duration::from_secs(2)))?;

        Ok(Self { tcp_stream })
    }
}

impl Com for ComRawImpl {
    fn get_name(&self) -> &'static str {
        "Raw"
    }

    fn default_port(&self) -> u16 {
        0
    }

    fn set_terminal_type(&mut self, _terminal: crate::addresses::Terminal) {}

    fn read_data(&mut self) -> TermComResult<Option<Vec<u8>>> {
        let mut buf = [0; 1024 * 256];
        if self.tcp_stream.peek(&mut buf)? == 0 {
            return Ok(None);
        }

        self.tcp_stream.set_nonblocking(false)?;
        match self.tcp_stream.read(&mut buf) {
            Ok(size) => {
                self.tcp_stream.set_nonblocking(true)?;
                Ok(Some(buf[0..size].to_vec()))
            }
            Err(ref e) => {
                self.tcp_stream.set_nonblocking(true)?;
                if e.kind() == io::ErrorKind::WouldBlock {
                    return Ok(None);
                }
                Err(Box::new(io::Error::new(
                    ErrorKind::ConnectionAborted,
                    format!("Connection aborted: {e}"),
                )))
            }
        }
    }

    fn send(&mut self, buf: &[u8]) -> TermComResult<usize> {
        self.tcp_stream.write_all(buf)?;
        Ok(buf.len())
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        self.tcp_stream.shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}
