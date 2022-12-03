use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use std::{fs};

pub mod xymodem;
use async_trait::async_trait;
use directories::UserDirs;
use rfd::FileDialog;
pub use xymodem::*;

pub mod zmodem;
pub use zmodem::*;
use crate::com::{Com, ComResult};
use crate::{com::{Connection}};

#[derive(Debug, Clone)]
pub struct FileDescriptor {
    pub path_name: String,
    pub file_name: String,
    pub size: usize,
    pub date: u64,
    path: PathBuf,
    data: Option<Vec<u8>>,
}

impl FileDescriptor {
    pub fn new() -> Self {
        Self {
            path_name: String::new(),
            file_name: String::new(),
            size: 0,
            date: 0,
            path: PathBuf::new(),
            data: None,
        }
    }

    pub fn from_paths(paths: &Vec<PathBuf>) -> ComResult<Vec<FileDescriptor>> {
        let mut res = Vec::new();
        for p in paths {
            let fd = FileDescriptor::create(p)?;
            res.push(fd);
        }
        Ok(res)
    }


    pub fn create(path: &PathBuf) -> ComResult<Self> {
        let data = fs::metadata(path)?;
        let size = data.len() as usize;
        let date = data
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        Ok(Self {
            path_name: path.to_str().unwrap().to_string(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path: path.clone(),
            size,
            date: date.as_secs(),
            data: None,
        })
    }

    #[cfg(test)]
    pub fn create_test(file_name: String, data: Vec<u8>) -> Self {
        Self {
            path_name: String::new(),
            file_name,
            path: PathBuf::new(),
            size: data.len(),
            date: 0,
            data: Some(data),
        }
    }

    pub fn get_data(&self) -> ComResult<Vec<u8>> {
        if let Some(data) = &self.data {
            Ok(data.clone())
        } else {
            let res = std::fs::read(&self.path);

            match res {
                Ok(res) => Ok(res),
                Err(err) => {
                    eprintln!("error {}", err);
                    Ok(Vec::new())
                }
            }
            
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransferInformation {
    pub file_name: String,
    pub file_size: usize,
    pub bytes_transfered: usize,

    pub errors: usize,
    pub files_finished: Vec<String>,
    pub check_size: String,
    time: SystemTime,
    bytes_transferred_timed: usize,
    pub bps: u64,

    pub output_log: Vec<String>,
}

impl TransferInformation {
    pub fn new() -> Self {
        Self {
            file_name: String::new(),
            file_size: 0,
            bytes_transfered: 0,
            errors: 0,
            files_finished: Vec::new(),
            check_size: String::new(),
            time: SystemTime::now(),
            output_log: Vec::new(),
            bytes_transferred_timed: 0,
            bps: 0,
        }
    }

    pub fn update_bps(&mut self) {
        let bytes = self
            .bytes_transfered
            .saturating_sub(self.bytes_transferred_timed);
        let length = SystemTime::now().duration_since(self.time).unwrap();

        if length > Duration::from_secs(10) {
            self.bytes_transferred_timed = self.bytes_transfered;
        }

        let length = length.as_secs() as usize;

        if length > 0 {
            self.bps = self.bps / 2 + (bytes / length) as u64;
        }

        let length = SystemTime::now().duration_since(self.time).unwrap();
        if length > Duration::from_secs(5) {
            self.bytes_transferred_timed = self.bytes_transfered;
            self.time = SystemTime::now();
        }
    }

    pub fn get_bps(&self) -> u64 {
        self.bps
    }

    pub fn write(&mut self, txt: String) {
        self.output_log.push(txt);
    }
}

#[derive(Debug, Clone)]
pub struct TransferState {
    pub current_state: &'static str,
    pub is_finished: bool,
    pub protocol_name: String,
    pub start_time: SystemTime,
    pub send_state: TransferInformation,
    pub recieve_state: TransferInformation,
}

impl TransferState {
    pub fn new() -> Self {
        Self {
            current_state: "",
            protocol_name: String::new(),
            is_finished: false,
            start_time: SystemTime::now(),
            send_state: TransferInformation::new(),
            recieve_state: TransferInformation::new()
        }
    }
}

#[async_trait]
pub trait Protocol: Send {
    async fn update(&mut self, com: &mut Box<dyn Com>, transfer_state: Arc<Mutex<TransferState>>) -> ComResult<bool>;

    async fn initiate_send(
        &mut self,
        com: &mut Box<dyn Com>,
        files: Vec<FileDescriptor>,
        transfer_state: Arc<Mutex<TransferState>>
    ) -> ComResult<()>;

    async fn initiate_recv(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: Arc<Mutex<TransferState>>
    ) -> ComResult<()>;

    fn get_received_files(&mut self) -> Vec<FileDescriptor>;

    async fn cancel(&mut self, com: &mut Box<dyn Com>) -> ComResult<()>;
}

#[derive(Debug, Clone, Copy)]
pub enum ProtocolType {
    ZModem,
    ZedZap,
    XModem,
    XModem1k,
    XModem1kG,
    YModem,
    YModemG,
}

impl ProtocolType {
    pub fn create(&self) -> Box<dyn Protocol> {
        match self {
            ProtocolType::ZModem => Box::new(Zmodem::new(1024)),
            ProtocolType::ZedZap => Box::new(Zmodem::new(8 * 1024)),
            ProtocolType::XModem => Box::new(XYmodem::new(XYModemVariant::XModem)),
            ProtocolType::XModem1k => Box::new(XYmodem::new(XYModemVariant::XModem1k)),
            ProtocolType::XModem1kG => Box::new(XYmodem::new(XYModemVariant::XModem1kG)),
            ProtocolType::YModem => Box::new(XYmodem::new(XYModemVariant::YModem)),
            ProtocolType::YModemG => Box::new(XYmodem::new(XYModemVariant::YModemG)),
        }
    }
}

pub fn str_from_null_terminated_utf8_unchecked(s: &[u8]) -> String {
    let mut res = String::new();

    for b in s {
        if *b == 0 {
            break;
        }
        res.push(char::from_u32(*b as u32).unwrap());
    }
    res
}
