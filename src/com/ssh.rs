#![allow(dead_code)]

use super::{Com, TermComResult};
use crate::address_mod::Address;
use async_trait::async_trait;
use libssh_rs::{Channel, Session, SshOption};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
pub struct SSHCom {
    session: Option<Session>,
    channel: Option<Arc<Mutex<Channel>>>,
}

impl SSHCom {
    pub fn new() -> Self {
        Self {
            session: None,
            channel: None,
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
        let sess = Session::new()?;
        sess.set_auth_callback(move |prompt, echo, verify, identity| Ok("<pw>".to_string()));
        sess.set_option(SshOption::Hostname("<addr>".to_string()))?;
        sess.set_option(SshOption::Port(2020))?;
        sess.set_option(SshOption::User(Some("<user>".to_string())))?;
        sess.set_option(SshOption::PublicKeyAcceptedTypes(
            "ssh-rsa,rsa-sha2-256,ssh-dss,ecdh-sha2-nistp256".to_string(),
        ))?;
        sess.options_parse_config(None)?;
        sess.connect()?;
        let auth_methods = sess.userauth_list(Some(&addr.user_name))?;

        sess.userauth_agent(Some(&addr.user_name))?;
        let chan = sess.new_channel()?;
        let mut result = [0u8; 1024];
        let size: usize = chan.read_timeout(&mut result, false, None)?;

        self.channel = Some(Arc::new(Mutex::new(chan)));
        self.session = Some(sess);
        Ok(true)
    }

    async fn read_data(&mut self) -> TermComResult<Vec<u8>> {
        Ok(vec![])
    }

    async fn read_u8(&mut self) -> TermComResult<u8> {
        Ok(0)
    }

    async fn read_exact(&mut self, _len: usize) -> TermComResult<Vec<u8>> {
        Ok(Vec::new())
    }

    async fn send<'a>(&mut self, buf: &'a [u8]) -> TermComResult<usize> {
        Ok(0)
    }

    async fn disconnect(&mut self) -> TermComResult<()> {
        Ok(())
    }
}

/* Trushh:
#![allow(dead_code)]

use super::{Com, TermComResult};
use crate::address_mod::Address;
use async_trait::async_trait;
use std::{collections::VecDeque, sync::Arc, time::Duration};
use thrussh::{
    client::{self, Channel},
    ChannelId, Disconnect,
};
use thrussh_keys::key;

pub struct SSHCom {
    channel: Option<Channel>,
    cur_data: VecDeque<u8>,
    inner: Option<client::Handle<Client>>,
}

impl SSHCom {
    pub fn new() -> Self {
        Self {
            channel: None,
            cur_data: VecDeque::new(),
            inner: None,
        }
    }
}
struct Client {}

impl client::Handler for Client {
    type Error = thrussh::Error;
    type FutureUnit = futures::future::Ready<Result<(Self, client::Session), Self::Error>>;
    type FutureBool = futures::future::Ready<Result<(Self, bool), Self::Error>>;

    fn finished_bool(self, b: bool) -> Self::FutureBool {
        futures::future::ready(Ok((self, b)))
    }

    fn finished(self, session: client::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, session)))
    }

    fn check_server_key(self, server_public_key: &key::PublicKey) -> Self::FutureBool {
        println!("check server key: {:?}", server_public_key);
        self.finished_bool(true)
    }

    fn auth_banner(self, banner: &str, session: client::Session) -> Self::FutureUnit {
        println!("--------");
        println!("{banner}");

        self.finished(session)
    }

    fn channel_open_confirmation(
        self,
        id: ChannelId,
        _max_packet_size: u32,
        _window_size: u32,
        session: client::Session,
    ) -> Self::FutureUnit {
        println!("channel_open_confirmation: {id:?}");
        self.finished(session)
    }

    fn data(self, _channel: ChannelId, data: &[u8], session: client::Session) -> Self::FutureUnit {
        println!("got data : {data:?}");
        self.finished(session)
    }
}

#[async_trait]
impl Com for SSHCom {
    fn get_name(&self) -> &'static str {
        "SSH"
    }
    fn set_terminal_type(&mut self, _terminal: crate::address_mod::Terminal) {}

    async fn connect(&mut self, addr: &Address, _timeout: Duration) -> TermComResult<bool> {
        println!("connect!!!");
        let config = thrussh::client::Config::default();
        let config = Arc::new(config);
        let handler = Client {};
        let mut session = thrussh::client::connect(config, &addr.address, handler)
            .await
            .unwrap();
        let auth_res = session
            .authenticate_password(&addr.user_name, &addr.password)
            .await?;
        println!("authenticate result: {}", auth_res);

        let channel = session.channel_open_session().await.unwrap();

        self.channel = Some(channel);
        self.inner = Some(session);
        Ok(true)
    }

    async fn read_data(&mut self) -> TermComResult<Vec<u8>> {
        if let Some(msg) = self.channel.as_mut().unwrap().wait().await {
            if let thrussh::ChannelMsg::Data { ref data } = msg {
                return Ok(data.to_vec());
            }
        }

        Ok(Vec::new())
    }

    async fn read_u8(&mut self) -> TermComResult<u8> {
        Ok(0)
    }

    async fn read_exact(&mut self, _len: usize) -> TermComResult<Vec<u8>> {
        Ok(Vec::new())
    }

    async fn send<'a>(&mut self, buf: &'a [u8]) -> TermComResult<usize> {
        self.channel.as_mut().unwrap().data(buf).await?;
        Ok(buf.len())
    }

    async fn disconnect(&mut self) -> TermComResult<()> {
        self.inner
            .as_mut()
            .unwrap()
            .disconnect(Disconnect::ByApplication, "", "English")
            .await?;
        Ok(())
    }
}
    */
