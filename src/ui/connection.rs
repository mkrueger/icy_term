use std::{collections::VecDeque, sync::mpsc};

use web_time::{Duration, Instant};

use crate::{Address, Terminal, TerminalResult};

/// Connection is used for the ui and com thread to communicate.
#[derive(Debug)]
pub struct Connection {
    connection_time: Instant,
    is_connected: bool,
    pub rx: mpsc::Receiver<SendData>,
    pub tx: mpsc::Sender<SendData>,
    end_transfer: bool,
    buf: std::collections::VecDeque<u8>,
}

impl Connection {
    pub fn new(rx: mpsc::Receiver<SendData>, tx: mpsc::Sender<SendData>) -> Self {
        Self {
            connection_time: Instant::now(),
            is_connected: false,
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
            self.is_connected = false;
            self.disconnect()?;
        }
        Ok(())
    }

    pub fn update_state(&mut self) -> TerminalResult<()> {
        self.fill_buffer()
    }

    fn fill_buffer(&mut self) -> TerminalResult<()> {
        loop {
            match self.rx.try_recv() {
                Ok(data) => match data {
                    SendData::Data(v) => {
                        self.buf.extend(v);
                    }
                    SendData::Disconnect => {
                        self.is_connected = false;
                        break;
                    }
                    SendData::EndTransfer => {
                        self.end_transfer = true;
                        break;
                    }
                    SendData::Connected => {
                        self.is_connected = true;
                        break;
                    }
                    SendData::ConnectionError(err) => {
                        self.is_connected = false;
                        self.end_transfer = true;
                        return Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            err,
                        )));
                    }
                    _ => {}
                },

                Err(err) => match err {
                    mpsc::TryRecvError::Empty => break,
                    mpsc::TryRecvError::Disconnected => {
                        self.is_connected = false;
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
        !self.is_connected
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
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

    pub(crate) fn connect(
        &self,
        call_adr: &Address,
        timeout: Duration,
        window_size: icy_engine::Size<u16>,
    ) -> TerminalResult<()> {
        self.tx
            .send(SendData::OpenConnection(OpenConnectionData::from(
                call_adr,
                timeout,
                window_size,
            )))?;
        Ok(())
    }
}

/// A more lightweight version of `Address` that is used for the connection
///Using Addreess in `SendData` makes just the enum larger without adding any value.
#[derive(Debug, Clone)]
pub struct OpenConnectionData {
    pub address: String,
    pub terminal: Terminal,
    pub user_name: String,
    pub password: String,
    pub protocol: crate::Protocol,
    pub timeout: Duration,
    pub window_size: icy_engine::Size<u16>,
}

impl OpenConnectionData {
    pub fn from(call_adr: &Address, timeout: Duration, window_size: icy_engine::Size<u16>) -> Self {
        Self {
            address: call_adr.address.clone(),
            user_name: call_adr.user_name.clone(),
            password: call_adr.password.clone(),
            terminal: call_adr.terminal_type,
            protocol: call_adr.protocol,
            timeout,
            window_size,
        }
    }
}

/// Data that is sent to the connection thread
#[derive(Debug)]
pub enum SendData {
    OpenConnection(OpenConnectionData),
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
