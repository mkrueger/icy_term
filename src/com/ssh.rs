use crate::address::Address;

use super::Com;
#[allow(dead_code)]
use async_trait::async_trait;
use russh::client;
use std::{io::ErrorKind, thread, time::Duration, sync::Arc, net::SocketAddr, str::FromStr};
use tokio::{
    io::{self, AsyncReadExt},
    net::TcpStream,
};

pub struct SSHCom {
    tcp_stream: TcpStream,
    buf: std::collections::VecDeque<u8>,
}

impl russh::client::Handler for SSHCom {
    type Error = russh::Error;
    type FutureUnit = futures::future::Ready<Result<(Self, client::Session), Self::Error>>;
    type FutureBool = futures::future::Ready<Result<(Self, bool), Self::Error>>;

    fn finished_bool(self, b: bool) -> Self::FutureBool {
        futures::future::ready(Ok((self, b)))
    }
    fn finished(self, session: client::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, session)))
    }
}

impl SSHCom {
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
impl Com for SSHCom {
    fn get_name(&self) -> &'static str {
        "OpenSSL"
    }

    async fn connect(&mut self, addr: &Address, timeout: Duration) -> Result<bool, String> {

        let config = russh::client::Config::default();
        let config = Arc::new(config);
    /* 
        let mut agent = AgentClient::connect_env()
            .await
            .unwrap();
        let mut identities = agent.request_identities().await.unwrap();*/
        let mut session =
            russh::client::connect(config, SocketAddr::from_str("127.0.0.1:2200").unwrap(), self)
                .await
                .unwrap();
        let (_, auth_res) = session
            .authenticate_future("pe", identities.pop().unwrap(), agent)
            .await;
        let auth_res = auth_res.unwrap();
        println!("=== auth: {}", auth_res);
        let mut channel = session
            .channel_open_direct_tcpip("localhost", 8000, "localhost", 3333)
            .await
            .unwrap();

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
