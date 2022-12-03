use crate::{address::Address, TerminalResult};

use super::{Com, ComResult };
#[allow(dead_code)]
use async_trait::async_trait;
use std::{time::Duration};
use tokio::{
    io::{ AsyncWriteExt, AsyncReadExt},
    net::TcpStream,
};

pub struct RawCom {
    tcp_stream: Option<TcpStream>
}

impl RawCom {
    pub fn new() -> Self {
        Self { tcp_stream: None }
    }
}

#[async_trait]
impl Com for RawCom {
    fn get_name(&self) -> &'static str {
        "Raw"
    }

    async fn connect(&mut self, addr: &Address, timeout: Duration) -> TerminalResult<bool> {
        let r = tokio::time::timeout(timeout, TcpStream::connect(&addr.address)).await;
        match r {
            Ok(tcp_stream) => match tcp_stream {
                Ok(stream) => {
                    self.tcp_stream = Some(stream);
                    Ok(true)
                }
                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    async fn read_data(&mut self) -> ComResult<Vec<u8>> {
        let mut buf = [0; 1024 * 50];
        match self.tcp_stream.as_mut().unwrap().read(&mut buf).await {
            Ok(bytes) => Ok(buf[0..bytes].into()),
            Err(err) => Err(Box::new(err))
        }
    }

    async fn read_u8(&mut self) -> ComResult<u8> {
        match self.tcp_stream.as_mut().unwrap().read_u8().await {
            Ok(b) => Ok(b),
            Err(err) => Err(Box::new(err))
        }
    }
    
    async fn read_exact(&mut self, len: usize) -> ComResult<Vec<u8>>{
        let mut buf = Vec::new();
        buf.resize(len, 0);
        match self.tcp_stream.as_mut().unwrap().read_exact(&mut buf).await {
            Ok(_b) => Ok(buf),
            Err(err) => Err(Box::new(err))
        }
    }

    async fn send<'a>(&mut self, buf: &'a [u8]) -> ComResult<usize> {
        match self.tcp_stream.as_mut().unwrap().write(&buf).await {
            Ok(bytes) => Ok(bytes),
            Err(err) => Err(Box::new(err))
        }
    }

    fn disconnect(&mut self) -> ComResult<()> {
        Ok(())
    }
}
