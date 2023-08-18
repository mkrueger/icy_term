#![allow(dead_code)]

use crate::addresses::Address;

use super::{Com, TermComResult};
use std::{
    io::{self, ErrorKind, Read, Write},
    net::TcpStream,
    time::Duration,
};

pub struct ComRawImpl {
    tcp_stream: Option<TcpStream>,
}

impl ComRawImpl {
    pub fn new() -> Self {
        Self { tcp_stream: None }
    }
}

impl Com for ComRawImpl {
    fn get_name(&self) -> &'static str {
        "Raw"
    }
    fn set_terminal_type(&mut self, _terminal: crate::addresses::Terminal) {}

    fn connect(&mut self, addr: &Address, timeout: Duration) -> TermComResult<bool> {
        let tcp_stream = TcpStream::connect(&addr.address)?;
        tcp_stream.set_nonblocking(true)?;
        tcp_stream.set_read_timeout(Some(Duration::from_secs(2)))?;

        self.tcp_stream = Some(tcp_stream);
        Ok(true)
    }

    fn read_data(&mut self) -> TermComResult<Option<Vec<u8>>> {
        let tcp_stream = self.tcp_stream.as_mut().unwrap();
        let mut buf = [0; 1024 * 256];
        if tcp_stream.peek(&mut buf)? == 0 {
            return Ok(None);
        }

        tcp_stream.set_nonblocking(false)?;
        match tcp_stream.read(&mut buf) {
            Ok(size) => {
                tcp_stream.set_nonblocking(true)?;
                Ok(Some(buf[0..size].to_vec()))
            }
            Err(ref e) => {
                tcp_stream.set_nonblocking(true)?;
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

    fn read_u8(&mut self) -> TermComResult<u8> {
        self.tcp_stream.as_mut().unwrap().set_nonblocking(false)?;
        let mut b = [0];
        match self.tcp_stream.as_mut().unwrap().read_exact(&mut b) {
            Ok(_) => {
                self.tcp_stream.as_mut().unwrap().set_nonblocking(true)?;
                Ok(b[0])
            }
            Err(err) => {
                self.tcp_stream.as_mut().unwrap().set_nonblocking(true)?;
                Err(Box::new(io::Error::new(
                    ErrorKind::ConnectionAborted,
                    format!("error while reading single byte from stream: {err}"),
                )))
            }
        }
    }

    fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>> {
        self.tcp_stream.as_mut().unwrap().set_nonblocking(false)?;
        let mut b = vec![0; len];
        self.tcp_stream.as_mut().unwrap().read_exact(&mut b)?;
        self.tcp_stream.as_mut().unwrap().set_nonblocking(true)?;

        Ok(b)
    }

    fn send(&mut self, buf: &[u8]) -> TermComResult<usize> {
        self.tcp_stream.as_mut().unwrap().write_all(buf)?;
        Ok(buf.len())
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        self.tcp_stream
            .as_mut()
            .unwrap()
            .shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }
}
