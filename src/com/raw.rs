use crate::address::Address;

use super::Com;
#[allow(dead_code)]
use async_trait::async_trait;
use std::{io::ErrorKind, thread, time::Duration, collections::VecDeque};
use tokio::{
    io::{self, AsyncWriteExt, AsyncReadExt},
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

    async fn read_data(&mut self) -> io::Result<Vec<u8>> {
        let mut buf = [0; 1024 * 50];
        let bytes = self.tcp_stream.as_mut().unwrap().read(&mut buf).await?;
        Ok(buf[0..bytes].into())
    }

    async fn write<'a>(&mut self, buf: &'a [u8]) -> io::Result<usize> {
        Ok(self.tcp_stream.as_mut().unwrap().write(&buf).await?)
    }

    fn disconnect(&mut self) -> io::Result<()> {
        Ok(())
    }
}
