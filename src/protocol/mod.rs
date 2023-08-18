#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

pub mod file_storage_handler;
pub use file_storage_handler::*;

pub mod xymodem;
pub use xymodem::*;

pub mod zmodem;
use crate::com::{Com, TermComResult};
pub use zmodem::*;

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
    pub fn from_paths(paths: &Vec<PathBuf>) -> TermComResult<Vec<FileDescriptor>> {
        let mut res = Vec::new();
        for p in paths {
            let fd = FileDescriptor::create(p)?;
            res.push(fd);
        }
        Ok(res)
    }

    pub fn create(path: &PathBuf) -> TermComResult<Self> {
        let data = fs::metadata(path)?;
        let size = usize::try_from(data.len()).unwrap();
        let date_duration = data
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();

        Ok(Self {
            path_name: path.to_str().unwrap().to_string(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path: path.clone(),
            size,
            date: date_duration.as_secs(),
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

    pub fn get_data(&self) -> std::vec::Vec<u8> {
        if let Some(data) = &self.data {
            data.clone()
        } else {
            let res = std::fs::read(&self.path);

            match res {
                Ok(res) => res,
                Err(err) => {
                    eprintln!("error {err}");
                    Vec::new()
                }
            }
        }
    }
}

impl Default for FileDescriptor {
    fn default() -> Self {
        Self {
            path_name: String::new(),
            file_name: String::new(),
            size: 0,
            date: 0,
            path: PathBuf::new(),
            data: None,
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
    pub fn update_bps(&mut self) {
        let bytes = self
            .bytes_transfered
            .saturating_sub(self.bytes_transferred_timed);
        let length = SystemTime::now().duration_since(self.time).unwrap();

        if length > Duration::from_secs(10) {
            self.bytes_transferred_timed = self.bytes_transfered;
        }

        let length = length.as_secs();
        if length > 0 {
            self.bps = self.bps / 2 + bytes as u64 / length;
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

    pub fn _write(&mut self, txt: String) {
        self.output_log.push(txt);
    }
}

impl Default for TransferInformation {
    fn default() -> Self {
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

impl Default for TransferState {
    fn default() -> Self {
        Self {
            current_state: "",
            protocol_name: String::new(),
            is_finished: false,
            start_time: SystemTime::now(),
            send_state: TransferInformation::default(),
            recieve_state: TransferInformation::default(),
        }
    }
}

pub trait Protocol: Send {
    fn update(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: &mut TransferState,
        storage_handler: &mut dyn FileStorageHandler,
    ) -> TermComResult<bool>;

    fn initiate_send(
        &mut self,
        com: &mut Box<dyn Com>,
        files: Vec<FileDescriptor>,
        transfer_state: &mut TransferState,
    ) -> TermComResult<()>;

    fn initiate_recv(
        &mut self,
        com: &mut Box<dyn Com>,
        transfer_state: &mut TransferState,
    ) -> TermComResult<()>;

    fn cancel(&mut self, com: &mut Box<dyn Com>) -> TermComResult<()>;
}

#[derive(Debug, Clone, Copy)]
pub enum TransferType {
    ZModem,
    ZedZap,
    XModem,
    XModem1k,
    XModem1kG,
    YModem,
    YModemG,
}

impl TransferType {
    pub fn create(self) -> Box<dyn Protocol> {
        match self {
            TransferType::ZModem => Box::new(Zmodem::new(1024)),
            TransferType::ZedZap => Box::new(Zmodem::new(8 * 1024)),
            TransferType::XModem => Box::new(XYmodem::new(XYModemVariant::XModem)),
            TransferType::XModem1k => Box::new(XYmodem::new(XYModemVariant::XModem1k)),
            TransferType::XModem1kG => Box::new(XYmodem::new(XYModemVariant::XModem1kG)),
            TransferType::YModem => Box::new(XYmodem::new(XYModemVariant::YModem)),
            TransferType::YModemG => Box::new(XYmodem::new(XYModemVariant::YModemG)),
        }
    }
}

pub fn str_from_null_terminated_utf8_unchecked(s: &[u8]) -> String {
    let mut res = String::new();

    for b in s {
        if *b == 0 {
            break;
        }
        res.push(char::from_u32(u32::from(*b)).unwrap());
    }
    res
}
