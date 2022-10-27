pub mod xymodem_core;
use std::path::{ PathBuf};
use std::time::SystemTime;
use std::{io, fs};

pub use xymodem_core::*;
pub use xymodem_core::*;

pub mod xmodem;
pub use xmodem::*;

pub mod ymodem;
pub use ymodem::*;

pub mod zmodem;
pub use zmodem::*;

use crate::com::Com;

#[derive(Clone)]
pub struct FileDescriptor {
    pub path_name: String,
    pub file_name: String,
    pub size: usize,
    pub date: u64,
    path: PathBuf,
    data: Option<Vec<u8>>
}

impl FileDescriptor {
    pub fn new() -> Self {
        Self {
            path_name: String::new(),
            file_name: String::new(),
            size: 0,
            date: 0,
            path: PathBuf::new(),
            data: None
        }
    }
    
    pub fn create(path: &PathBuf) -> io::Result<Self> {
        let data = fs::metadata(path)?;
        let size = data.len() as usize;
        let date = data.modified()?.duration_since(SystemTime::UNIX_EPOCH).unwrap();
        
        Ok(Self {
            path_name: path.to_str().unwrap().to_string(),
            file_name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path: path.clone(),
            size,
            date: date.as_secs(),
            data: None
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
            data: Some(data)
        }
    }

    pub fn get_data(&self) -> io::Result<Vec<u8>>
    {
        if let Some(data) = &self.data {
            Ok(data.clone())
        } else {
            let res = std::fs::read(&self.path)?;
            Ok(res)
        }
    }
}

#[derive(Clone)]
pub struct FileTransferState {
    pub file: Option<FileDescriptor>,
    pub bytes_transfered: usize,
    pub errors: usize,
    pub files_finished: Vec<String>
}

impl FileTransferState {
    pub fn new() -> Self {
        Self {
            file: None,
            bytes_transfered: 0,
            errors: 0,
            files_finished: Vec::new(),
        }
    }
}


#[derive(Clone)]
pub struct TransferState {
    pub cur_check: String,
    pub send_state: Option<FileTransferState>,
    pub recieve_state: Option<FileTransferState>,
}

impl TransferState {
    pub fn new() -> Self {
        Self {
            cur_check: String::new(),
            send_state: None,
            recieve_state: None
        }
    }
}

pub trait Protocol
{
    fn get_current_state(&self) -> Option<TransferState>;

    fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>;

    fn initiate_send<T: Com>(&mut self, com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>;
    fn initiate_recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>;

    fn get_received_files(&mut self) -> Vec<FileDescriptor>;
}
