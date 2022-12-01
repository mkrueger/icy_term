use std::{
    time::{Duration, SystemTime}, error::Error, fmt::Display, thread, collections::VecDeque,
};

#[cfg(test)]
pub mod test_com;
use async_trait::async_trait;
#[cfg(test)]
pub use test_com::*;

pub mod telnet;
pub use telnet::*;

pub mod raw;
pub use raw::*;

pub mod ssh;
pub use ssh::*;
use tokio::sync::mpsc;

use crate::{address::Address, TerminalResult};

pub type ComResult<T> = Result<T, Box<dyn Error + Send>>;

#[async_trait]
pub trait Com: Sync + Send {
    fn get_name(&self) -> &'static str;

    async fn write<'a>(&mut self, buf: &'a [u8]) -> ComResult<usize>;
    async fn connect(&mut self, addr: &Address, timeout: Duration) -> TerminalResult<bool>;
    async fn read_data(&mut self) -> ComResult<Vec<u8>>;

    fn disconnect(&mut self) -> ComResult<()>;
}


#[derive(Debug, Clone)]
pub enum ComError {
    Timeout
}

impl Display for ComError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ComError {
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

unsafe impl Send for ComError {
    
}

#[derive(Debug)]
pub enum SendData {
    Data(Vec<u8>),
    Disconnect
}

pub struct Connection {
    connection_time: SystemTime,
    is_disconnected: bool,
    rx: mpsc::Receiver<SendData>,
    tx: mpsc::Sender<SendData>,

    buf: std::collections::VecDeque<u8>,
}

impl Connection {
    pub fn new(rx: mpsc::Receiver<SendData>, tx: mpsc::Sender<SendData>) -> Self {
        Self {
            connection_time: SystemTime::now(),
            is_disconnected: false,
            rx,
            tx,
            buf: VecDeque::new()
        }
    }

    pub fn get_connection_time(&self) -> SystemTime {
        self.connection_time
    }

    pub fn send(&mut self, vec: Vec<u8>) -> TerminalResult<()> {
        if let Err(err) = self.tx.try_send(SendData::Data(vec)) {
            self.is_disconnected = true;
            eprintln!("{}", err);
        }
        Ok(())
    }
    
    fn fill_buffer(&mut self) -> TerminalResult<()> {
        loop {
            match self.rx.try_recv() {
                Ok(data) => {
                    match data {
                        SendData::Data(v) => {
                            self.buf.extend(v);
                        },
                        SendData::Disconnect =>  {
                            self.is_disconnected = true;
                            break;
                        },
                    }
                }
    
                Err(err) => {
                    match err {
                        mpsc::error::TryRecvError::Empty => break,
                        mpsc::error::TryRecvError::Disconnected => { 
                            self.is_disconnected = true;
                            return Err(Box::new(err));
                        },
                    }
                }
            }
        }
        Ok(())
    }
    
    fn fill_buffer_wait(&mut self, timeout: Duration) -> TerminalResult<()>  {
        self.fill_buffer()?;
        let now = SystemTime::now();
        while self.buf.len() == 0 {
            self.fill_buffer()?;
            thread::sleep(Duration::from_millis(10));
            if SystemTime::now().duration_since(now)? > timeout {
                return Err(Box::new(ComError::Timeout));
            }
        }
        Ok(())
    }

    pub fn is_data_available(&mut self) -> TerminalResult<bool> {
        self.fill_buffer()?;
        Ok(self.buf.len() > 0)
    }
    
    pub fn read_char(&mut self, duration: Duration) -> TerminalResult<u8> {
        self.fill_buffer()?;
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        self.fill_buffer_wait(duration)?;
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        return Err(Box::new(ComError::Timeout));
    }

    pub fn read_exact(&mut self, duration: Duration, bytes: usize) -> TerminalResult<Vec<u8>> {
        while self.buf.len() < bytes {
            self.fill_buffer_wait(duration)?;
        }
        Ok(self.buf.drain(0..bytes).collect())
    }

    pub fn read_buffer(&mut self) -> TerminalResult<Vec<u8>> {
        Ok(self.buf.drain(0..self.buf.len()).collect())
    }
    
    pub fn disconnect(&self) -> TerminalResult<()> {
        self.tx.try_send(SendData::Disconnect)?;
        Ok(())
    }

    pub fn is_disconnected(&self) -> bool {
        self.is_disconnected
    }
}