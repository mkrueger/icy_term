use std::{
    collections::VecDeque,
    error::Error,
    time::{Duration, SystemTime},
};

#[cfg(test)]
pub mod tests;
use async_trait::async_trait;
#[cfg(test)]
pub use tests::*;

pub mod telnet;
pub use telnet::*;

pub mod raw;
pub use raw::*;

use tokio::sync::mpsc;

use crate::{address_mod::Address, TerminalResult};
pub type TermComResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[async_trait]
pub trait Com: Sync + Send {
    fn get_name(&self) -> &'static str;

    async fn send<'a>(&mut self, buf: &'a [u8]) -> TermComResult<usize>;
    async fn connect(&mut self, addr: &Address, timeout: Duration) -> TermComResult<bool>;
    async fn read_data(&mut self) -> TermComResult<Vec<u8>>;
    async fn read_u8(&mut self) -> TermComResult<u8>;
    async fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>>;

    fn disconnect(&mut self) -> TermComResult<()>;
}

#[derive(Debug)]
pub enum SendData {
    Data(Vec<u8>),
    Disconnect,

    StartTransfer(
        crate::protocol::TransferType,
        bool,
        std::sync::Arc<std::sync::Mutex<crate::protocol::TransferState>>,
        Option<Vec<crate::protocol::FileDescriptor>>,
    ),
    EndTransfer,
    CancelTransfer,
}

#[derive(Debug)]
pub struct Connection {
    connection_time: SystemTime,
    is_disconnected: bool,
    pub rx: mpsc::Receiver<SendData>,
    pub tx: mpsc::Sender<SendData>,
    end_transfer: bool,

    buf: std::collections::VecDeque<u8>,
}

impl Connection {
    pub fn new(rx: mpsc::Receiver<SendData>, tx: mpsc::Sender<SendData>) -> Self {
        Self {
            connection_time: SystemTime::now(),
            is_disconnected: false,
            end_transfer: false,
            rx,
            tx,
            buf: VecDeque::new(),
        }
    }

    pub fn should_end_transfer(&mut self) -> bool {
        self.fill_buffer().unwrap_or_default();
        self.end_transfer
    }

    pub fn get_connection_time(&self) -> SystemTime {
        self.connection_time
    }

    pub fn send(&mut self, vec: Vec<u8>) -> TerminalResult<()> {
        if let Err(err) = self.tx.try_send(SendData::Data(vec)) {
            eprintln!("{err}");
            self.is_disconnected = true;
            self.disconnect()?;
        }
        Ok(())
    }

    fn fill_buffer(&mut self) -> TerminalResult<()> {
        loop {
            match self.rx.try_recv() {
                Ok(data) => match data {
                    SendData::Data(v) => {
                        self.buf.extend(v);
                    }
                    SendData::Disconnect => {
                        self.is_disconnected = true;
                        break;
                    }
                    SendData::EndTransfer => {
                        self.end_transfer = true;
                        break;
                    }
                    _ => {}
                },

                Err(err) => match err {
                    mpsc::error::TryRecvError::Empty => break,
                    mpsc::error::TryRecvError::Disconnected => {
                        self.is_disconnected = true;
                        return Err(Box::new(err));
                    }
                },
            }
        }
        Ok(())
    }

    pub fn is_data_available(&mut self) -> TerminalResult<bool> {
        self.fill_buffer()?;
        Ok(!self.buf.is_empty())
    }

    pub fn read_buffer(&mut self) -> Vec<u8> {
        self.buf.drain(0..self.buf.len()).collect()
    }

    pub fn disconnect(&self) -> TerminalResult<()> {
        self.tx.try_send(SendData::Disconnect)?;
        Ok(())
    }

    pub fn cancel_transfer(&self) -> TerminalResult<()> {
        self.tx.try_send(SendData::CancelTransfer)?;
        Ok(())
    }

    pub fn is_disconnected(&self) -> bool {
        self.is_disconnected
    }

    pub(crate) fn start_file_transfer(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        state: std::sync::Arc<std::sync::Mutex<crate::protocol::TransferState>>,
        files_opt: Option<Vec<crate::protocol::FileDescriptor>>,
    ) -> TerminalResult<()> {
        self.end_transfer = false;
        self.tx.try_send(SendData::StartTransfer(
            protocol_type,
            download,
            state,
            files_opt,
        ))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectionError {
    ConnectionLost,
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::ConnectionLost => {
                write!(f, "connection lost")
            }
        }
    }
}

impl Error for ConnectionError {
    fn description(&self) -> &str {
        "use std::display"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}
