#![allow(dead_code)]

use super::{Com, TermComResult};
use crate::address_mod::Address;
use async_trait::async_trait;
use ssh_rs::{ssh, SessionConnector, ShellBrocker};
use std::{collections::VecDeque, net::TcpStream, time::Duration};

static mut TCP_STREAM: Option<ShellBrocker> = None;

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
    fn set_terminal_type(&mut self, _terminal: crate::address_mod::Terminal) {}

    async fn connect(&mut self, addr: &Address, _timeout: Duration) -> TermComResult<bool> {
        match ssh::create_session()
            .private_key_path("/home/mkrueger/.ssh/id_rsa")
            .username(&addr.user_name)
            .password(&addr.password)
            .connect(&addr.address)
        {
            Ok(session) => {
                let mut broker = session.run_backend();
                let shell = broker.open_shell().unwrap();

                unsafe {
                    TCP_STREAM = Some(shell);
                }
                Ok(true)
            }
            Err(err) => Err(Box::new(err)),
        }
    }

    async fn read_data(&mut self) -> TermComResult<Vec<u8>> {
        unsafe {
            match TCP_STREAM.as_mut().unwrap().read() {
                Ok(data) => Ok(data),
                Err(err) => Err(Box::new(err)),
            }
        }
    }

    async fn read_u8(&mut self) -> TermComResult<u8> {
        if self.cur_data.is_none() {
            let data = self.read_data().await?;
            self.cur_data = Some(VecDeque::from_iter(data));
        }

        if let Some(d) = &mut self.cur_data {
            let result = d.pop_front();
            if d.is_empty() {
                self.cur_data = None;
            }

            return Ok(result.unwrap());
        }
        Ok(0)
    }

    async fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>> {
        if self.cur_data.is_none() {
            let data = self.read_data().await?;
            self.cur_data = Some(VecDeque::from_iter(data));
        }

        if let Some(d) = &mut self.cur_data {
            let result = d.drain(..len).collect();

            if d.is_empty() {
                self.cur_data = None;
            }
            return Ok(result);
        }
        Ok(Vec::new())
    }

    async fn send<'a>(&mut self, buf: &'a [u8]) -> TermComResult<usize> {
        unsafe {
            match TCP_STREAM.as_mut().unwrap().write(buf) {
                Ok(_) => Ok(buf.len()),
                Err(err) => Err(Box::new(err)),
            }
        }
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        Ok(())
    }
}
