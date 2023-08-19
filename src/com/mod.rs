use std::{collections::VecDeque, error::Error, sync::mpsc, time::Duration};

#[cfg(test)]
pub mod tests;
#[cfg(test)]
pub use tests::*;

pub mod telnet;
pub use telnet::*;

pub mod raw;
pub use raw::*;
use web_time::Instant;

// pub mod ssh;

use crate::{
    addresses::{Address, Terminal},
    TerminalResult,
};
pub type TermComResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub trait Com: Sync + Send {
    fn get_name(&self) -> &'static str;

    fn send(&mut self, buf: &[u8]) -> TermComResult<usize>;
    fn connect(&mut self, addr: &Address, timeout: Duration) -> TermComResult<bool>;
    fn read_data(&mut self) -> TermComResult<Option<Vec<u8>>>;
    fn read_u8(&mut self) -> TermComResult<u8>;
    fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>>;
    fn set_terminal_type(&mut self, terminal: Terminal);

    fn disconnect(&mut self) -> TermComResult<()>;
}
pub struct NullConnection {}
impl Com for NullConnection {
    fn get_name(&self) -> &'static str {
        ""
    }

    fn send(&mut self, buf: &[u8]) -> TermComResult<usize> {
        Ok(0)
    }

    fn connect(&mut self, addr: &Address, timeout: Duration) -> TermComResult<bool> {
        Ok(false)
    }

    fn read_data(&mut self) -> TermComResult<Option<Vec<u8>>> {
        Ok(Some(Vec::new()))
    }

    fn read_u8(&mut self) -> TermComResult<u8> {
        Ok(0)
    }

    fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>> {
        Ok(Vec::new())
    }

    fn set_terminal_type(&mut self, terminal: Terminal) {}

    fn disconnect(&mut self) -> TermComResult<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub enum SendData {
    OpenConnection(Address, Duration, icy_engine::Size<u16>),
    ConnectionError(String),
    Connected,

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
    SetBaudRate(u32),
}

#[derive(Debug)]
pub struct Connection {
    connection_time: Instant,
    is_disconnected: bool,
    pub rx: mpsc::Receiver<SendData>,
    pub tx: mpsc::Sender<SendData>,
    end_transfer: bool,

    buf: std::collections::VecDeque<u8>,
}

impl Connection {
    pub fn new(rx: mpsc::Receiver<SendData>, tx: mpsc::Sender<SendData>) -> Self {
        Self {
            connection_time: Instant::now(),
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

    pub fn get_connection_time(&self) -> Instant {
        self.connection_time
    }

    pub fn send(&mut self, vec: Vec<u8>) -> TerminalResult<()> {
        if let Err(err) = self.tx.send(SendData::Data(vec)) {
            log::error!("Error sending data: {err}");
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
                    SendData::Connected => {
                        self.is_disconnected = false;
                        break;
                    }
                    SendData::ConnectionError(err) => {
                        log::error!("Connection error: {}", err);
                        self.is_disconnected = true;
                        self.end_transfer = true;
                        break;
                    }
                    _ => {}
                },

                Err(err) => match err {
                    mpsc::TryRecvError::Empty => break,
                    mpsc::TryRecvError::Disconnected => {
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
        self.tx.send(SendData::Disconnect)?;
        Ok(())
    }

    pub fn cancel_transfer(&self) -> TerminalResult<()> {
        self.tx.send(SendData::CancelTransfer)?;
        Ok(())
    }

    pub fn is_disconnected(&self) -> bool {
        self.is_disconnected
    }

    pub fn is_connected(&self) -> bool {
        !self.is_disconnected
    }

    pub(crate) fn start_file_transfer(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        state: std::sync::Arc<std::sync::Mutex<crate::protocol::TransferState>>,
        files_opt: Option<Vec<crate::protocol::FileDescriptor>>,
    ) -> TerminalResult<()> {
        self.end_transfer = false;
        self.tx.send(SendData::StartTransfer(
            protocol_type,
            download,
            state,
            files_opt,
        ))?;
        Ok(())
    }

    pub(crate) fn set_baud_rate(&self, baud_rate: u32) -> TerminalResult<()> {
        self.tx.send(SendData::SetBaudRate(baud_rate))?;
        Ok(())
    }

    pub(crate) fn Connect(
        &self,
        call_adr: Address,
        timeout: Duration,
        window_size: icy_engine::Size<u16>,
    ) -> TerminalResult<()> {
        log::info!("Connecting to {:?}", call_adr);
        self.tx
            .send(SendData::OpenConnection(call_adr, timeout, window_size))?;
        log::info!("Connected done");
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
