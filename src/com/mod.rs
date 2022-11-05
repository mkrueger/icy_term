use std::{time::Duration, io::{self}};

#[cfg(test)]
pub mod test_com;
#[cfg(test)]
pub use test_com::*;

pub mod telnet;
pub use telnet::*;

pub trait Com {
    fn get_name(&self) -> &'static str;

    fn read_char(&mut self, duration: Duration) -> io::Result<u8>;
    fn read_char_nonblocking(&mut self) -> io::Result<u8>;
    fn read_exact(&mut self, duration: Duration, bytes: usize) -> io::Result<Vec<u8>>;
    
    fn is_data_available(&mut self) -> io::Result<bool>;

    fn write(&mut self, buf: &[u8]) -> io::Result<()>;

    fn disconnect(&mut self) -> io::Result<()>;
}