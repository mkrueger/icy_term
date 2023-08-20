#![allow(dead_code)]
use super::{Com, OpenConnectionData, TermComResult};
use ssh2::{Channel, Session};
use std::{
    io::ErrorKind,
    io::{Read, Write},
    net::TcpStream,
};
use web_time::Duration;
pub struct SSHComImpl {
    session: Session,
    channel: Channel,
}

impl SSHComImpl {
    pub fn connect(connection_data: &OpenConnectionData) -> TermComResult<Self> {
        let tcp_stream = TcpStream::connect(&connection_data.address)?;
        tcp_stream.set_read_timeout(Some(Duration::from_secs(2)))?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp_stream);
        session.handshake()?;

        //  :TODO: SECURITY: verify_known_hosts() implemented here -- ie: user must accept & we save somewhere
        session.userauth_password(&connection_data.user_name, &connection_data.password)?;
        let mut channel = session.channel_session()?;

        let terminal_type = connection_data.terminal.to_string().to_lowercase();
        channel.request_pty(
            terminal_type.as_str(),
            None,
            Some((
                connection_data.window_size.width as u32,
                connection_data.window_size.height as u32,
                0,
                0,
            )),
        )?;
        channel.shell()?;
        session.set_blocking(false);

        Ok(Self { session, channel })
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

    fn read_data(&mut self) -> TermComResult<Option<Vec<u8>>> {
        let mut buf = [0; 1024 * 256];

        match self.channel.read(&mut buf) {
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
        self.channel.write_all(buf)?;
        Ok(buf.len())
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        self.session.disconnect(
            Some(ssh2::DisconnectCode::ByApplication),
            "",
            Some("English"),
        )?;
        Ok(())
    }
}
