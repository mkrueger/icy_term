use super::{Com, ComResult};
use crate::address::Address;
#[allow(dead_code)]
use async_trait::async_trait;
use ssh_rs::{ssh, SessionConnector, ShellBrocker};
use std::{collections::VecDeque, net::TcpStream, time::Duration};

static mut tcp_stream: Option<ShellBrocker> = None;

pub struct SSHCom {
    session: Option<SessionConnector<TcpStream>>,
    cur_data: Option<VecDeque<u8>>,
}

impl SSHCom {
    pub fn new() -> Self {
        Self {
            session: None,
            cur_data: None,
        }
    }
}

#[async_trait]
impl Com for SSHCom {
    fn get_name(&self) -> &'static str {
        "SSH"
    }

    async fn connect(&mut self, addr: &Address, _timeout: Duration) -> ComResult<bool> {
        match ssh::create_session()
            .username(&addr.user_name)
            .password(&addr.password)
            .connect(&addr.address)
        {
            Ok(session) => {
                let mut broker = session.run_backend();
                let shell = broker.open_shell().unwrap();

                unsafe {
                    tcp_stream = Some(shell);
                }
                Ok(true)
            }
            Err(err) => Err(Box::new(err)),
        }
    }

    async fn read_data(&mut self) -> ComResult<Vec<u8>> {
        unsafe {
            match tcp_stream.as_mut().unwrap().read() {
                Ok(data) => Ok(data),
                Err(err) => Err(Box::new(err)),
            }
        }
    }

    async fn read_u8(&mut self) -> ComResult<u8> {
        if self.cur_data.is_none() {
            let data = self.read_data().await?;
            self.cur_data = Some(VecDeque::from_iter(data));
        }

        if let Some(d) = &mut self.cur_data {
            let result = d.pop_front();
            if d.len() == 0 {
                self.cur_data = None
            }

            return Ok(result.unwrap());
        }
        Ok(0)
    }

    async fn read_exact(&mut self, len: usize) -> ComResult<Vec<u8>> {
        if self.cur_data.is_none() {
            let data = self.read_data().await?;
            self.cur_data = Some(VecDeque::from_iter(data));
        }

        if let Some(d) = &mut self.cur_data {
            let result = d.drain(..len).collect();

            if d.len() == 0 {
                self.cur_data = None
            }
            return Ok(result);
        }
        Ok(Vec::new())
    }

    async fn send<'a>(&mut self, buf: &'a [u8]) -> ComResult<usize> {
        unsafe {
            match tcp_stream.as_mut().unwrap().write(buf) {
                Ok(_) => Ok(buf.len()),
                Err(err) => Err(Box::new(err)),
            }
        }
    }

    fn disconnect(&mut self) -> ComResult<()> {
        Ok(())
    }
}
