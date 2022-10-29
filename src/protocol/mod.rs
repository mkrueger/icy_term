use std::path::{ PathBuf};
use std::time::SystemTime;
use std::{io, fs};

pub mod xymodem;
pub use xymodem::*;

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

    pub fn from_paths(paths: &Vec<PathBuf>) -> io::Result<Vec<FileDescriptor>> {
        let mut res = Vec::new();
        for p in paths {
            let fd = FileDescriptor::create(p)?;
            res.push(fd);
        }
        Ok(res)
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
    pub files_finished: Vec<String>,
    pub check_size: String,
    pub engine_state: String,
}

impl FileTransferState {
    pub fn new() -> Self {
        Self {
            file: None,
            bytes_transfered: 0,
            errors: 0,
            files_finished: Vec::new(),
            check_size: String::new(),
            engine_state: String::new()
        }
    }

    pub fn get_file_name(&self) -> String {
        match &self.file {
            Some(file) => file.file_name.clone(),
            None => "<unknown>".to_string()
        }
    }

    pub fn _get_file_size(&self) -> usize {
        match &self.file {
            Some(file) => file.size,
            None => 0
        }
    }

    pub fn get_total_bytes(&self) -> usize {
        if let Some(f) = &self.file {
            f.size
        } else {
            0
        }
    }
}


#[derive(Clone)]
pub struct TransferState {
    pub current_state: &'static str,
    pub send_state: Option<FileTransferState>,
    pub recieve_state: Option<FileTransferState>,
}

impl TransferState {
    pub fn new() -> Self {
        Self {
            current_state: "",
            send_state: None,
            recieve_state: None
        }
    }
}

pub trait Protocol
{
    fn get_name(&self) -> &str;
    
    fn get_current_state(&self) -> Option<TransferState>;

    fn is_active(&self) -> bool;

    fn update<T: Com>(&mut self, com: &mut T) -> io::Result<()>;

    fn initiate_send<T: Com>(&mut self, com: &mut T, files: Vec<FileDescriptor>) -> io::Result<()>;
    fn initiate_recv<T: Com>(&mut self, com: &mut T) -> io::Result<()>;

    fn get_received_files(&mut self) -> Vec<FileDescriptor>;

    fn cancel<T: Com>(&mut self, com: &mut T) -> io::Result<()>;
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