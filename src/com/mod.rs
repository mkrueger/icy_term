use std::{
    io::{self},
    time::Duration,
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

use crate::address::Address;

#[async_trait]
pub trait Com: Sync + Send {
    fn get_name(&self) -> &'static str;

    fn read_char(&mut self, duration: Duration) -> io::Result<u8>;
    fn read_char_nonblocking(&mut self) -> io::Result<u8>;
    fn read_exact(&mut self, duration: Duration, bytes: usize) -> io::Result<Vec<u8>>;

    fn is_data_available(&mut self) -> io::Result<bool>;
    
    async fn write<'a>(&mut self, buf: &'a [u8]) -> io::Result<usize>;
    
    async fn connect(&mut self, addr: &Address, timeout: Duration) -> Result<bool, String>;
    async fn read_data(&mut self) -> io::Result<Vec<u8>>;

    fn disconnect(&mut self) -> io::Result<()>;
}
