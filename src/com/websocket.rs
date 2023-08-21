use crate::addresses;
use std::borrow::BorrowMut;
use std::sync::Arc;

use super::{Com, OpenConnectionData, TermComResult};

use rustls::{OwnedTrustAnchor, RootCertStore};
use tungstenite::{client::IntoClientRequest, WebSocket, stream::MaybeTlsStream, Error, Message};
use std::net::TcpStream;
use std::io::ErrorKind;
use http::Uri;

struct NoCertVerifier {}

impl rustls::client::ServerCertVerifier for NoCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        server_name: &rustls::ServerName,
        scts: &mut dyn Iterator<Item = &[u8]>,
        ocsp_response: &[u8],
        now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error>
    {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
pub struct WebSocketComImpl {
    socket: WebSocket<MaybeTlsStream<TcpStream>>
}


impl WebSocketComImpl {
    pub fn connect(connection_data: &OpenConnectionData) -> TermComResult<Self> {
        let is_secure = connection_data.protocol == addresses::Protocol::WebSocket(true);

        // build an ws:// or wss:// address
        //  :TODO: default port if not supplied in address
        let url = format!("{}://{}", Self::schema_prefix(is_secure), connection_data.address);
        let uri = Uri::try_from(url)?;

        let mut root_store = RootCertStore::empty();
        root_store.add_trust_anchors(
            webpki_roots::TLS_SERVER_ROOTS
                .iter()
                .map(|ta| {
                    OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                }),
        );

        let mut config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // enable this line to test non-secure (ie: invalid certs) wss:// -- we could make this an option in the UI
        //config.dangerous().set_certificate_verifier(Arc::new(NoCertVerifier{}));

        let config = Arc::new(config);

        let stream = TcpStream::connect(connection_data.address.clone())?;
        let connector: tungstenite::Connector = tungstenite::Connector::Rustls(config);
        let (mut socket, _) = tungstenite::client_tls_with_config(uri.into_client_request()?, stream, None, Some(connector))?;

        let s = socket.get_mut();
        match s {
            MaybeTlsStream::Plain(s) => {
                s.set_nonblocking(true)?;
            }
            MaybeTlsStream::Rustls(tls) => {
                tls.sock.set_nonblocking(true)?;
            },
            _ => ()
        }

        Ok(Self{socket})
    }

    fn schema_prefix(is_secure: bool) -> &'static str {
        match is_secure {
            true => "wss",
            false => "ws"
        }
    }
}

impl Com for WebSocketComImpl {
    fn get_name(&self) -> &'static str {
        "WebSocket"
    }

    fn default_port(&self) -> u16 {
        443 // generally secure by default
    }

    fn set_terminal_type(&mut self, _terminal: crate::addresses::Terminal) {}

    fn read_data(&mut self) -> TermComResult<Option<Vec<u8>>> {
        match self.socket.read() {
            Ok(msg) => Ok(Some(msg.into_data())),
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) =>
                Err(Box::new(std::io::Error::new(
                    ErrorKind::ConnectionAborted,
                    format!("Connection aborted: {e}"),
                )))
        }
    }

    fn read_u8(&mut self) -> TermComResult<u8> {
        Ok(0)
    }

    fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>> {
        Ok(vec![0])
    }

    fn send(&mut self, buf: &[u8]) -> TermComResult<usize> {
        let msg = Message::binary(buf);
        self.socket.send(msg)?; // write + flush
        Ok(buf.len())
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        Ok(self.socket.close(None)?)
    }
}