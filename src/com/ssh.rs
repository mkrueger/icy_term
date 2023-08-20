#![allow(dead_code)]

use super::{Com, OpenConnectionData, TermComResult};
use icy_engine::Size;
use libssh_rs::{Channel, Session, SshOption};
use std::{
    io::ErrorKind,
    io::{Read, Write},
    sync::{Arc, Mutex},
};
pub struct SSHComImpl {
    window_size: Size<u16>, // width, height
    session: Option<Session>,
    channel: Option<Arc<Mutex<Channel>>>,
}

impl SSHComImpl {
    pub fn new(window_size: Size<u16>) -> Self {
        Self {
            window_size,
            session: None,
            channel: None,
        }
    }

    fn default_port() -> u16 {
        22
    }

    fn parse_address(addr: &str) -> TermComResult<(String, u16)> {
        let components: Vec<&str> = addr.split(':').collect();
        match components.first() {
            Some(host) => match components.get(1) {
                Some(port_str) => {
                    let port = port_str.parse()?;
                    Ok(((*host).to_string(), port))
                }
                _ => Ok(((*host).to_string(), Self::default_port())),
            },
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid address",
            ))),
        }
    }
}

impl Com for SSHComImpl {
    fn get_name(&self) -> &'static str {
        "SSH"
    }

    fn default_port(&self) -> u16 {
        SSHComImpl::default_port()
    }

    fn set_terminal_type(&mut self, _terminal: crate::addresses::Terminal) {}

    fn connect(&mut self, connection_data: &OpenConnectionData) -> TermComResult<bool> {
        let sess = Session::new()?;
        let (host, port) = Self::parse_address(&connection_data.address)?;

        sess.set_option(SshOption::Hostname(host))?;
        sess.set_option(SshOption::Port(port))?;
        sess.options_parse_config(None)?;
        sess.connect()?;

        //  :TODO: SECURITY: verify_known_hosts() implemented here -- ie: user must accept & we save somewhere

        sess.userauth_password(
            Some(connection_data.user_name.as_str()),
            Some(connection_data.password.as_str()),
        )?;

        let chan = sess.new_channel()?;
        chan.open_session()?;
        let terminal_type = connection_data.terminal.to_string().to_lowercase();
        chan.request_pty(
            terminal_type.as_str(),
            self.window_size.width as u32,
            self.window_size.height as u32,
        )?;
        chan.request_shell()?;

        self.channel = Some(Arc::new(Mutex::new(chan)));
        self.session = Some(sess);

        Ok(true)
    }

    fn read_data(&mut self) -> TermComResult<Option<Vec<u8>>> {
        let channel = self.channel.as_mut().unwrap().clone();
        let mut buf = [0; 1024 * 256];
        let locked = channel.lock().unwrap();
        let mut stdout = locked.stdout();
        match stdout.read(&mut buf) {
            Ok(size) => Ok(Some(buf[0..size].to_vec())),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    return Ok(None);
                }
                Err(Box::new(std::io::Error::new(
                    ErrorKind::ConnectionAborted,
                    format!("Connection aborted: {e}"),
                )))
            }
        }
    }

    fn read_u8(&mut self) -> TermComResult<u8> {
        Ok(0)
    }

    fn read_exact(&mut self, _len: usize) -> TermComResult<Vec<u8>> {
        Ok(Vec::new())
    }

    fn send(&mut self, buf: &[u8]) -> TermComResult<usize> {
        let channel = self.channel.as_mut().unwrap().clone();
        let locked = channel.lock().unwrap();
        locked.stdin().write_all(buf)?;
        Ok(buf.len())
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        self.session.as_mut().unwrap().disconnect();
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
