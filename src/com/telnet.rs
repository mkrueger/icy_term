use std::{io::{ErrorKind, self, Read, Write}, time::Duration, net::{SocketAddr, TcpStream}, thread};
use super::Com;

pub struct TelnetCom
{
    tcp_stream: TcpStream,
    state: ParserState,
    buf: std::collections::VecDeque<u8>
}

enum ParserState {
    Data,
    Iac,
    Will,
    Wont,
    Do,
    Dont
}

mod iac_cmd {
    /// End of subnegotiation parameters.
    pub const SE:u8 = 0xF0;

    /// No operation.
    pub const NOP:u8 = 0xF1;

    /// The data stream portion of a Synch.
    /// This should always be accompanied
    /// by a TCP Urgent notification.
    pub const Data_Mark:u8 = 0xF2;

    /// NVT character BRK
    pub const Break:u8 = 0xF3;
    
    /// The function Interrupt Process
    pub const IP:u8 = 0xF4;

    // The function Abort output
    pub const AO:u8 = 0xF5;

    // The function Are You There
    pub const AYT:u8 = 0xF6;

    // The function Erase character
    pub const EC:u8 = 0xF7;

    // The function Erase line
    pub const EL:u8 = 0xF8;

    // The Go ahead signal.
    pub const GA:u8 = 0xF9;

    // Indicates that what follows is subnegotiation of the indicated option.
    pub const SB:u8 = 0xFA;

    ///  (option code)
    /// Indicates the desire to begin performing, or confirmation that you are now performing, the indicated option.
    pub const WILL:u8 = 0xFB;

    /// (option code) 
    /// Indicates the refusal to perform, or continue performing, the indicated option.
    pub const WONT:u8 = 0xFC; 

    /// (option code)
    /// Indicates the request that the other party perform, or confirmation that you are expecting
    /// the other party to perform, the indicated option.
    pub const DO:u8 = 0xFD;

    /// (option code) 
    /// Indicates the demand that the other party stop performing,
    /// or confirmation that you are no longer expecting the other party
    /// to perform, the indicated option.
    pub const DONT:u8 = 0xFE;

    /// Data Byte 255.
    pub const IAC:u8 = 0xFF;
}


#[allow(dead_code)]
mod options {
    /// https://www.rfc-editor.org/rfc/rfc856
    pub const TRANSMIT_BINARY:u8 = 0x00;
    /// https://www.rfc-editor.org/rfc/rfc857
    pub const ECHO:u8 = 0x01;
    /// https://www.rfc-editor.org/rfc/rfc858
    pub const SUPPRESS_GO_AHEAD:u8 = 0x03;
    /// https://www.rfc-editor.org/rfc/rfc859
    pub const STATUS:u8 = 0x05;
    /// https://www.rfc-editor.org/rfc/rfc860
    pub const TIMING_MARK:u8 = 0x06;
    /// https://www.rfc-editor.org/rfc/rfc861
    pub const EXTENDED_OPTIONS_LIST:u8 = 0xFF;
    /// https://www.rfc-editor.org/rfc/rfc885
    pub const END_OF_RECORD:u8 = 25;

    /// https://www.rfc-editor.org/rfc/rfc1073
    pub const NAWS:u8 = 31;
    /// https://www.rfc-editor.org/rfc/rfc1079
    pub const TERMINAL_SPEED:u8 = 32;
    /// https://www.rfc-editor.org/rfc/rfc1091
    pub const TERMINAL_TYPE:u8 = 24;
    /// https://www.rfc-editor.org/rfc/rfc1096
    pub const X_DISPLAY_LOCATION:u8 = 35;
    /// https://www.rfc-editor.org/rfc/rfc1184
    pub const LINEMODE:u8 = 34;
    /// https://www.rfc-editor.org/rfc/rfc1372
    pub const TOGGLE_FLOW_CONTROL:u8 = 33;
    /// https://www.rfc-editor.org/rfc/rfc1572
    pub const NEW_ENVIRON:u8 = 39;

    /// https://www.rfc-editor.org/rfc/rfc2941
    pub const AUTHENTICATION:u8 = 37;
    /// https://www.rfc-editor.org/rfc/rfc2946
    pub const ENCRYPT:u8 = 38;
}

impl TelnetCom 
{
    pub fn connect(addr: &SocketAddr, timeout: Duration) -> io::Result<Self> {
        let tcp_stream = std::net::TcpStream::connect_timeout(addr, timeout)?;
        tcp_stream.set_nonblocking(true)?;

        Ok(Self { 
            tcp_stream,
            state: ParserState::Data,
            buf: std::collections::VecDeque::new()
        })
    }

    fn parse(&mut self, data: &[u8]) -> io::Result<()>
    {
        for b in data {
            match self.state {
                ParserState::Data => {
                    if *b == iac_cmd::IAC {
                        self.state = ParserState::Iac;
                    } else {
                        self.buf.push_back(*b);
                    }
                },
                ParserState::Iac => {
                    match *b {
                        iac_cmd::AYT => {
                            self.tcp_stream.write_all(&[iac_cmd::IAC, iac_cmd::NOP])?;
                            self.state = ParserState::Data;
                        }
                        iac_cmd::SE |
                        iac_cmd::NOP |
                        iac_cmd::GA => { self.state = ParserState::Data; }
                        iac_cmd::IAC => {
                            self.buf.push_back(0xFF);
                            self.state = ParserState::Data;
                        }
                        iac_cmd::WILL => {
                            self.state = ParserState::Will;
                        }
                        iac_cmd::WONT => {
                            self.state = ParserState::Wont;
                        }
                        iac_cmd::DO => {
                            self.state = ParserState::Do;
                        }
                        iac_cmd::DONT => {
                            self.state = ParserState::Dont;
                        }
                        cmd => {
                            eprintln!("unknown IAC: {}/x{:02X}", cmd, cmd);
                            self.state = ParserState::Data;
                        }
                    }
                }
                ParserState::Will => {
                    if *b != options::TRANSMIT_BINARY {
                        self.tcp_stream.write_all(&[iac_cmd::IAC, iac_cmd::DONT, *b])?;
                    } else {
                        println!("unknown will option :{:02X}", *b);
                        self.tcp_stream.write_all(&[iac_cmd::IAC, iac_cmd::DO, options::TRANSMIT_BINARY])?;
                    }
                    self.state = ParserState::Data;
                },
                ParserState::Wont => {
                    println!("Won't {}", *b);
                    self.state = ParserState::Data;
                },
                ParserState::Do => {
                    if *b == options::TRANSMIT_BINARY {
                        self.tcp_stream.write_all(&[iac_cmd::IAC, iac_cmd::WILL, options::TRANSMIT_BINARY])?;
                    } else {
                        println!("unknown do option :{:02X}", *b);
                        self.tcp_stream.write_all(&[iac_cmd::IAC, iac_cmd::WONT, *b])?;
                    }
                    self.state = ParserState::Data;
                },
                ParserState::Dont => {
                    println!("Don't {}", *b);
                    self.state = ParserState::Data;
                },
            }
        }
        Ok(())
    }

    fn fill_buffer(&mut self) -> io::Result<()> {
        let mut buf = [0;1024 * 8];
        loop {
            match self.tcp_stream.read(&mut buf) {
                Ok(size) => {
                    return self.parse(&buf[0..size]);
                }
                Err(ref e) => {
                    if e.kind() == io::ErrorKind::WouldBlock {
                        break;
                    }
                    return Err(io::Error::new(ErrorKind::ConnectionAborted, format!("{}", e)));
                }
            };
        }
        Ok(())
    }

    fn fill_buffer_wait(&mut self, _timeout: Duration) -> io::Result<()> {
        self.tcp_stream.set_nonblocking(false)?;
        self.fill_buffer()?;
        while self.buf.len() == 0 {
            self.fill_buffer()?;
            thread::sleep(Duration::from_millis(10));
        }
        self.tcp_stream.set_nonblocking(true)?;
        Ok(())
    }
}

impl Com for TelnetCom {
    fn get_name(&self) -> &'static str {
        "Telnet"
    }

    fn read_char(&mut self, timeout: Duration) -> io::Result<u8> {
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        self.fill_buffer_wait(timeout)?;
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        return Err(io::Error::new(ErrorKind::TimedOut, "timed out"));
    }
    
    fn read_char_nonblocking(&mut self) -> io::Result<u8> {
        if let Some(b) = self.buf.pop_front() {
            return Ok(b);
        }
        return Err(io::Error::new(ErrorKind::TimedOut, "no data avaliable"));
    }

    fn read_exact(&mut self, duration: Duration, bytes: usize) -> io::Result<Vec<u8>> {
        while self.buf.len() < bytes {
            self.fill_buffer_wait(duration)?;
        }
        Ok(self.buf.drain(0..bytes).collect())
    }
    
    fn is_data_available(&mut self) -> io::Result<bool> {
        self.fill_buffer()?; 
        Ok(self.buf.len() > 0)
    }

    fn disconnect(&mut self) -> io::Result<()> {
        self.tcp_stream.shutdown(std::net::Shutdown::Both)
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<()> {
        let mut data = Vec::with_capacity(buf.len());
        for b in buf {
            if *b == iac_cmd::IAC {
                data.extend_from_slice(&[iac_cmd::IAC, iac_cmd::IAC]);
            } else {
                data.push(*b);
            }
        }
        self.tcp_stream.write_all(&data)
    }
}
