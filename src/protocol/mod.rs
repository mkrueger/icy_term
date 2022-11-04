use std::path::{ PathBuf};
use std::time::{SystemTime, Duration};
use std::{io, fs};

pub mod xymodem;
use directories::UserDirs;
use rfd::FileDialog;
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

    pub fn save_file_in_downloads(&self) -> io::Result<()> {

        if let Some(user_dirs) = UserDirs::new() { 
            let dir = user_dirs.download_dir().unwrap();

            if self.file_name.is_empty() {
                let new_name = FileDialog::new()
                .save_file();
                if let Some(path) = new_name {
                    let out_file = dir.join(path);
                    println!("Storing file as '{:?}'…", out_file);
                    fs::write(out_file, &self.get_data()?)?;
                }
                return Ok(());
            }
            let mut file_name = dir.join(&self.file_name);
            let mut i = 1;
            while file_name.exists() {
                file_name = dir.join(&format!("{}.{}", self.file_name, i));
                i += 1;
            }
            println!("Storing file as '{:?}'…", file_name);
            fs::write(file_name, &self.get_data()?)?;
        }
        Ok(())
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
    pub file_name: String,
    pub file_size: usize,
    pub bytes_transfered: usize,
    pub errors: usize,
    pub files_finished: Vec<String>,
    pub check_size: String,
    pub engine_state: String,
    time: SystemTime,
    bytes_transferred_timed: usize,
    bps: u64
}

impl FileTransferState {
    pub fn new() -> Self {
        Self {
            file_name: String::new(),
            file_size: 0,
            bytes_transfered: 0,
            errors: 0,
            files_finished: Vec::new(),
            check_size: String::new(),
            engine_state: String::new(),
            time: SystemTime::now(),
            bytes_transferred_timed: 0,
            bps: 0
        }
    }

    pub fn update_bps(&mut self) 
    {
        let bytes = self.bytes_transfered.saturating_sub(self.bytes_transferred_timed);
        let length = SystemTime::now().duration_since(self.time).unwrap();
    
        if length > Duration::from_secs(10) {
            self.bytes_transferred_timed = self.bytes_transfered;
        }
    
        let length = length.as_secs() as usize;

        if length > 0 {
            self.bps = self.bps / 2 +  (bytes / length) as u64;
        }

        let length = SystemTime::now().duration_since(self.time).unwrap();
        if length > Duration::from_secs(5) {
            self.bytes_transferred_timed = self.bytes_transfered;
            self.time = SystemTime::now();
        }
    }

    pub fn get_bps(&self) -> u64 { self.bps }
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
    fn get_current_state(&self) -> Option<&TransferState>;
    fn is_active(&self) -> bool;
    fn update(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>;
    fn initiate_send(&mut self, com: &mut Box<dyn Com>, files: Vec<FileDescriptor>) -> io::Result<()>;
    fn initiate_recv(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>;
    fn get_received_files(&mut self) -> Vec<FileDescriptor>;
    fn cancel(&mut self, com: &mut Box<dyn Com>) -> io::Result<()>;
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