use crate::address_mod::{Address, Terminal};

use super::{Com, ConnectionError, TermComResult};
use async_trait::async_trait;
use icy_engine::Size;
use std::{io::ErrorKind, time::Duration};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[derive(Debug)]
pub struct ComTelnetImpl {
    tcp_stream: Option<TcpStream>,
    state: ParserState,
    window_size: Size<u16>, // width, height
    terminal: Terminal,
    use_raw_transfer: bool,
}

#[derive(Debug)]
enum ParserState {
    Data,
    Iac,
    Will,
    Wont,
    Do,
    Dont,
    SubCommand(i32),
}

mod terminal_type {
    pub const IS: u8 = 0x00;
    pub const SEND: u8 = 0x01;
    // pub const MAXLN: usize = 40;
}

mod telnet_cmd {
    use crate::com::TermComResult;

    /// End of subnegotiation parameters.
    pub const SE: u8 = 0xF0;

    /// No operation.
    pub const Nop: u8 = 0xF1;

    /// The data stream portion of a Synch.
    /// This should always be accompanied
    /// by a TCP Urgent notification.
    pub const DataMark: u8 = 0xF2;

    /// NVT character BRK
    pub const Break: u8 = 0xF3;

    /// The function Interrupt Process
    pub const IP: u8 = 0xF4;

    // The function Abort output
    pub const AO: u8 = 0xF5;

    // The function Are You There
    pub const Ayt: u8 = 0xF6;

    // The function Erase character
    pub const EC: u8 = 0xF7;

    // The function Erase line
    pub const EL: u8 = 0xF8;

    // The Go ahead signal.
    pub const GA: u8 = 0xF9;

    // Indicates that what follows is subnegotiation of the indicated option.
    pub const SB: u8 = 0xFA;

    ///  (option code)
    /// Indicates the desire to begin performing, or confirmation that you are now performing, the indicated option.
    pub const Will: u8 = 0xFB;

    /// (option code)
    /// Indicates the refusal to perform, or continue performing, the indicated option.
    pub const Wont: u8 = 0xFC;

    /// (option code)
    /// Indicates the request that the other party perform, or confirmation that you are expecting
    /// the other party to perform, the indicated option.
    pub const DO: u8 = 0xFD;

    /// (option code)
    /// Indicates the demand that the other party stop performing,
    /// or confirmation that you are no longer expecting the other party
    /// to perform, the indicated option.
    pub const Dont: u8 = 0xFE;

    /// Data Byte 255.
    pub const Iac: u8 = 0xFF;

    pub fn make_cmd(byte: u8) -> [u8; 2] {
        [Iac, byte]
    }

    pub fn make_cmd_with_option(byte: u8, option: u8) -> [u8; 3] {
        [Iac, byte, option]
    }

    pub fn check(byte: u8) -> TermComResult<u8> {
        match byte {
            0xF0..=0xFF => Ok(byte),
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown IAC: {byte}/x{byte:02X}"),
            ))),
        }
    }

    pub fn to_string(byte: u8) -> &'static str {
        match byte {
            SE => "SE",
            Nop => "Nop",
            DataMark => "DataMark",
            Break => "Break",
            IP => "IP",
            AO => "AO",
            Ayt => "Ayt",
            EC => "EC",
            EL => "EL",
            GA => "GA",
            SB => "SB",
            Will => "Will",
            Wont => "Wont",
            DO => "DO",
            Dont => "Dont",
            Iac => "Iac",
            _ => "unknown",
        }
    }
}

/**
<http://www.iana.org/assignments/telnet-options/telnet-options.xhtml>
*/
mod telnet_option {
    /// <https://www.rfc-editor.org/rfc/rfc856>
    pub const TransmitBinary: u8 = 0x00;
    /// <https://www.rfc-editor.org/rfc/rfc857>
    pub const Echo: u8 = 0x01;
    /// ???
    pub const Reconnection: u8 = 0x02;
    /// <https://www.rfc-editor.org/rfc/rfc858>
    pub const SuppressGoAhead: u8 = 0x03;
    /// <https://www.rfc-editor.org/rfc/rfc859>
    pub const Status: u8 = 0x05;
    /// <https://www.rfc-editor.org/rfc/rfc860>
    pub const TimingMark: u8 = 0x06;
    /// <https://www.rfc-editor.org/rfc/rfc726.html>
    pub const RemoteControlledTransAndEcho: u8 = 0x07;
    /// ???
    pub const OutputLineWidth: u8 = 0x08;
    /// ???
    pub const OutputPageSize: u8 = 0x09;
    ///<https://www.rfc-editor.org/rfc/RFC652>
    pub const OutputCarriageReturnDisposition: u8 = 10;
    ///<https://www.rfc-editor.org/rfc/RFC653>
    pub const OutputHorizontalTabStops: u8 = 11;
    ///<https://www.rfc-editor.org/rfc/RFC654>
    pub const OutputHorizontalTabDisposition: u8 = 12;
    ///<https://www.rfc-editor.org/rfc/RFC655>
    pub const OutputFormfeedDisposition: u8 = 13;
    ///<https://www.rfc-editor.org/rfc/RFC656>
    pub const OutputVerticalTabstops: u8 = 14;
    ///<https://www.rfc-editor.org/rfc/RFC657>
    pub const OutputVerticalTabDisposition: u8 = 15;
    ///<https://www.rfc-editor.org/rfc/RFC658>
    pub const OutputLinefeedDisposition: u8 = 16;
    ///<https://www.rfc-editor.org/rfc/RFC698>
    pub const ExtendedASCII: u8 = 17;
    ///<https://www.rfc-editor.org/rfc/RFC727>
    pub const Logout: u8 = 18;
    ///<https://www.rfc-editor.org/rfc/RFC735>
    pub const ByteMacro: u8 = 19;
    ///<https://www.rfc-editor.org/rfc/RFC1043][RFC732>
    pub const DataEntryTerminal: u8 = 20;
    ///<https://www.rfc-editor.org/rfc/RFC736][RFC734>
    pub const SupDup: u8 = 21;
    ///<https://www.rfc-editor.org/rfc/RFC749>
    pub const SupDupOutput: u8 = 22;
    ///<https://www.rfc-editor.org/rfc/RFC779>
    pub const SendLocation: u8 = 23;
    /// <https://www.rfc-editor.org/rfc/rfc1091>
    pub const TerminalType: u8 = 24;
    /// <https://www.rfc-editor.org/rfc/rfc885>
    pub const EndOfRecord: u8 = 25;
    /// <https://www.rfc-editor.org/rfc/rfc1073>
    pub const NegotiateAboutWindowSize: u8 = 31;
    /// <https://www.rfc-editor.org/rfc/rfc1079>
    pub const TerminalSpeed: u8 = 32;
    /// <https://www.rfc-editor.org/rfc/rfc1372>
    pub const ToggleFlowControl: u8 = 33;
    /// <https://www.rfc-editor.org/rfc/rfc1184>
    pub const LineMode: u8 = 34;
    /// <https://www.rfc-editor.org/rfc/rfc1096>
    pub const XDisplayLocation: u8 = 35;
    /// <https://www.rfc-editor.org/rfc/rfc1408>
    pub const EnvironmentOption: u8 = 36;
    /// <https://www.rfc-editor.org/rfc/rfc2941>
    pub const Authentication: u8 = 37;
    /// <https://www.rfc-editor.org/rfc/rfc2946>
    pub const Encrypt: u8 = 38;
    /// <https://www.rfc-editor.org/rfc/rfc1572>
    pub const NewEnviron: u8 = 39;
    ///<https://www.rfc-editor.org/rfc/RFC2355>
    pub const TN3270E: u8 = 40;
    ///<https://www.rfc-editor.org/rfc/Rob_Earhart>
    pub const XAuth: u8 = 41;
    ///<https://www.rfc-editor.org/rfc/RFC2066>
    pub const CharSet: u8 = 42;
    ///<https://www.rfc-editor.org/rfc/Robert_Barnes>
    pub const TelnetRemoteSerialPortRSP: u8 = 43;
    ///<https://www.rfc-editor.org/rfc/RFC2217>
    pub const ComPortControlOption: u8 = 44;
    ///<https://www.rfc-editor.org/rfc/Wirt_Atmar>
    pub const TelnetSuppressLocalEcho: u8 = 45;
    ///<https://www.rfc-editor.org/rfc/Michael_Boe>
    pub const TelnetStartTLS: u8 = 46;
    ///<https://www.rfc-editor.org/rfc/RFC2840>
    pub const Kermit: u8 = 47;
    ///<https://www.rfc-editor.org/rfc/David_Croft>
    pub const SendURL: u8 = 48;
    ///<https://www.rfc-editor.org/rfc/Jeffrey_Altman>
    pub const ForwardX: u8 = 49;
    // 50-137 	Unassigned
    pub const TelOptPragmaLogon: u8 = 138;
    ///<https://www.rfc-editor.org/rfc/Steve_McGregory>
    pub const TelOptSSPILogon: u8 = 139;
    ///<https://www.rfc-editor.org/rfc/Steve_McGregory>
    pub const TelOptPragmaHeartbeat: u8 = 140;
    ///<https://www.rfc-editor.org/rfc/Steve_McGregory>
    // 141-254 	Unassigned
    /// <https://www.rfc-editor.org/rfc/rfc861>
    pub const ExtendedOptionsList: u8 = 0xFF;

    pub fn check(byte: u8) -> crate::com::TermComResult<u8> {
        match byte {
            0..=49 | 138..=140 | 255 => Ok(byte),
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("unknown option: {byte}/x{byte:02X}"),
            ))),
        }
    }

    pub fn to_string(byte: u8) -> &'static str {
        match byte {
            TransmitBinary => "TransmitBinary",
            Echo => "Echo",
            Reconnection => "Reconnection",
            SuppressGoAhead => "SuppressGoAhead",
            Status => "Status",
            TimingMark => "TimingMark",
            RemoteControlledTransAndEcho => "RemoteControlledTransAndEcho",
            OutputLineWidth => "OutputLineWidth",
            OutputPageSize => "OutputPageSize",
            OutputCarriageReturnDisposition => "OutputCarriageReturnDisposition",
            OutputHorizontalTabStops => "OutputHorizontalTabStops",
            OutputHorizontalTabDisposition => "OutputHorizontalTabDisposition",
            OutputFormfeedDisposition => "OutputFormfeedDisposition",
            OutputVerticalTabstops => "OutputVerticalTabstops",
            OutputVerticalTabDisposition => "OutputVerticalTabDisposition",
            OutputLinefeedDisposition => "OutputLinefeedDisposition",
            ExtendedASCII => "ExtendedASCII",
            Logout => "Logout",
            ByteMacro => "ByteMacro",
            DataEntryTerminal => "DataEntryTerminal",
            SupDup => "SupDup",
            SupDupOutput => "SupDupOutput",
            SendLocation => "SendLocation",
            TerminalType => "TerminalType",
            EndOfRecord => "EndOfRecord",
            NegotiateAboutWindowSize => "NegotiateAboutWindowSize",
            TerminalSpeed => "TerminalSpeed",
            ToggleFlowControl => "ToggleFlowControl",
            LineMode => "LineMode",
            XDisplayLocation => "XDisplayLocation",
            EnvironmentOption => "EnvironmentOption",
            Authentication => "Authentication",
            Encrypt => "Encrypt",
            NewEnviron => "NewEnviron",
            TN3270E => "TN3270E",
            XAuth => "XAuth",
            CharSet => "CharSet",
            TelnetRemoteSerialPortRSP => "TelnetRemoteSerialPortRSP",
            ComPortControlOption => "ComPortControlOption",
            TelnetSuppressLocalEcho => "TelnetSuppressLocalEcho",
            TelnetStartTLS => "TelnetStartTLS",
            Kermit => "Kermit",
            SendURL => "SendURL",
            ForwardX => "ForwardX",
            TelOptPragmaLogon => "TelOptPragmaLogon",
            TelOptSSPILogon => "TelOptSSPILogon",
            TelOptPragmaHeartbeat => "TelOptPragmaHeartbeat",
            ExtendedOptionsList => "ExtendedOptionsList",
            _ => "Unknown",
        }
    }
}

#[allow(dead_code)]
impl ComTelnetImpl {
    pub fn new(window_size: Size<u16>) -> Self {
        Self {
            tcp_stream: None,
            state: ParserState::Data,
            window_size,
            terminal: Terminal::Ansi,
            use_raw_transfer: false,
        }
    }

    fn parse(&mut self, data: &[u8]) -> TermComResult<Vec<u8>> {
        if self.use_raw_transfer {
            return Ok(data.to_vec());
        }
        let mut buf = Vec::with_capacity(data.len());
        for b in data {
            match self.state {
                ParserState::Data => {
                    if *b == telnet_cmd::Iac {
                        self.state = ParserState::Iac;
                    } else {
                        buf.push(*b);
                    }
                }

                ParserState::SubCommand(cmd) => {
                    match *b {
                        telnet_cmd::Iac => {}
                        telnet_cmd::SE => {
                            self.state = ParserState::Data;
                        }
                        terminal_type::SEND => {
                            // Send
                            if cmd == telnet_option::TerminalType as i32 {
                                let mut buf: Vec<u8> = vec![
                                    telnet_cmd::Iac,
                                    telnet_cmd::SB,
                                    telnet_option::TerminalType,
                                    terminal_type::IS,
                                ];

                                match self.terminal {
                                    Terminal::Ansi => buf.extend_from_slice(b"ANSI"),
                                    Terminal::PETscii => buf.extend_from_slice(b"PETSCII"),
                                    Terminal::ATAscii => buf.extend_from_slice(b"ATASCII"),
                                    Terminal::ViewData => buf.extend_from_slice(b"VIEWDATA"),
                                    Terminal::Ascii => buf.extend_from_slice(b"RAW"),
                                    Terminal::Avatar => buf.extend_from_slice(b"AVATAR"),
                                }
                                buf.extend([telnet_cmd::Iac, telnet_cmd::SE]);

                                //println!("Sending terminal type: {:?}", buf);

                                if let Some(stream) = self.tcp_stream.as_mut() {
                                    stream.try_write(&buf)?;
                                } else {
                                    return Err(Box::new(ConnectionError::ConnectionLost));
                                }
                            }
                        }
                        24 => {
                            // Ternminal type
                            self.state =
                                ParserState::SubCommand(telnet_option::TerminalType as i32);
                        }
                        _ => {}
                    }
                }
                ParserState::Iac => match telnet_cmd::check(*b) {
                    Ok(telnet_cmd::Ayt) => {
                        self.state = ParserState::Data;
                        if let Some(stream) = self.tcp_stream.as_mut() {
                            stream.try_write(&telnet_cmd::make_cmd(telnet_cmd::Nop))?;
                        } else {
                            return Err(Box::new(ConnectionError::ConnectionLost));
                        }
                    }
                    Ok(telnet_cmd::SE | telnet_cmd::Nop | telnet_cmd::GA) => {
                        self.state = ParserState::Data;
                    }
                    Ok(telnet_cmd::Iac) => {
                        buf.push(0xFF);
                        self.state = ParserState::Data;
                    }
                    Ok(telnet_cmd::Will) => {
                        self.state = ParserState::Will;
                    }
                    Ok(telnet_cmd::Wont) => {
                        self.state = ParserState::Wont;
                    }
                    Ok(telnet_cmd::DO) => {
                        self.state = ParserState::Do;
                    }
                    Ok(telnet_cmd::Dont) => {
                        self.state = ParserState::Dont;
                    }
                    Ok(telnet_cmd::SB) => {
                        self.state = ParserState::SubCommand(-1);
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        self.state = ParserState::Data;
                    }
                    Ok(cmd) => {
                        eprintln!("unsupported IAC: {}", telnet_cmd::to_string(cmd));
                        self.state = ParserState::Data;
                    }
                },
                ParserState::Will => {
                    self.state = ParserState::Data;
                    let opt = telnet_option::check(*b)?;
                    if let Some(stream) = self.tcp_stream.as_mut() {
                        if let telnet_option::TransmitBinary = opt {
                            stream.try_write(&telnet_cmd::make_cmd_with_option(
                                telnet_cmd::DO,
                                telnet_option::TransmitBinary,
                            ))?;
                        } else if let telnet_option::Echo = opt {
                            stream.try_write(&telnet_cmd::make_cmd_with_option(
                                telnet_cmd::DO,
                                telnet_option::Echo,
                            ))?;
                        } else if let telnet_option::SuppressGoAhead = opt {
                            stream.try_write(&telnet_cmd::make_cmd_with_option(
                                telnet_cmd::DO,
                                telnet_option::SuppressGoAhead,
                            ))?;
                        } else {
                            eprintln!("unsupported will option {}", telnet_option::to_string(opt));
                            stream.try_write(&telnet_cmd::make_cmd_with_option(
                                telnet_cmd::Dont,
                                opt,
                            ))?;
                        }
                    } else {
                        return Err(Box::new(ConnectionError::ConnectionLost));
                    }
                }
                ParserState::Wont => {
                    let opt = telnet_option::check(*b)?;
                    eprintln!("Won't {opt:?}");
                    self.state = ParserState::Data;
                }
                ParserState::Do => {
                    self.state = ParserState::Data;
                    let opt = telnet_option::check(*b)?;
                    if let Some(stream) = self.tcp_stream.as_mut() {
                        match opt {
                            telnet_option::TransmitBinary => {
                                stream.try_write(&telnet_cmd::make_cmd_with_option(
                                    telnet_cmd::Will,
                                    telnet_option::TransmitBinary,
                                ))?;
                            }
                            telnet_option::TerminalType => {
                                stream.try_write(&telnet_cmd::make_cmd_with_option(
                                    telnet_cmd::Will,
                                    telnet_option::TerminalType,
                                ))?;
                            }
                            telnet_option::NegotiateAboutWindowSize => {
                                // NAWS: send our current window size
                                let mut buf: Vec<u8> = telnet_cmd::make_cmd_with_option(
                                    telnet_cmd::SB,
                                    telnet_option::NegotiateAboutWindowSize,
                                )
                                .to_vec();
                                buf.extend(self.window_size.width.to_be_bytes());
                                buf.extend(self.window_size.height.to_be_bytes());
                                buf.push(telnet_cmd::Iac);
                                buf.push(telnet_cmd::SE);

                                stream.try_write(&buf)?;
                            }
                            _ => {
                                eprintln!(
                                    "unsupported do option {}",
                                    telnet_option::to_string(opt)
                                );
                                stream.try_write(&telnet_cmd::make_cmd_with_option(
                                    telnet_cmd::Wont,
                                    opt,
                                ))?;
                            }
                        }
                    } else {
                        return Err(Box::new(ConnectionError::ConnectionLost));
                    }
                }
                ParserState::Dont => {
                    let opt = telnet_option::check(*b)?;
                    eprintln!("Don't {opt:?}");
                    self.state = ParserState::Data;
                }
            }
        }
        Ok(buf)
    }
}

#[async_trait]
impl Com for ComTelnetImpl {
    fn get_name(&self) -> &'static str {
        "Telnet"
    }
    fn set_terminal_type(&mut self, terminal: crate::address_mod::Terminal) {
        self.terminal = terminal;
    }

    async fn connect(&mut self, addr: &Address, timeout: Duration) -> TermComResult<bool> {
        let mut addr_copy = addr.address.clone();
        if !addr_copy.contains(':') {
            addr_copy.push_str(":23");
        }
        let r = tokio::time::timeout(timeout, TcpStream::connect(&addr_copy)).await;
        match r {
            Ok(tcp_stream) => match tcp_stream {
                Ok(stream) => {
                    self.tcp_stream = Some(stream);
                    Ok(true)
                }
                Err(err) => Err(Box::new(err)),
            },
            Err(err) => Err(Box::new(err)),
        }
    }

    async fn read_data(&mut self) -> TermComResult<Vec<u8>> {
        let mut buf = [0; 1024 * 50];
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.read(&mut buf).await {
                Ok(bytes) => {
                    //                    println!("read {} bytes: {:?}", bytes, &buf[0..bytes]);
                    self.parse(&buf[0..bytes])
                }
                Err(error) => Err(Box::new(error)),
            }
        } else {
            return Err(Box::new(io::Error::new(ErrorKind::BrokenPipe, "no stream")));
        }
    }

    async fn read_u8(&mut self) -> TermComResult<u8> {
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.read_u8().await {
                Ok(b) => {
                    if b == telnet_cmd::Iac {
                        let b2 = stream.read_u8().await?;
                        if b2 != telnet_cmd::Iac {
                            return Err(Box::new(io::Error::new(
                                ErrorKind::InvalidData,
                                format!("expected iac, got {b2}"),
                            )));
                        }
                        // IGNORE additional telnet commands
                    }
                    Ok(b)
                }
                Err(err) => Err(Box::new(err)),
            }
        } else {
            return Err(Box::new(ConnectionError::ConnectionLost));
        }
    }

    async fn read_exact(&mut self, len: usize) -> TermComResult<Vec<u8>> {
        let mut buf = vec![0; len];
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.read_exact(&mut buf).await {
                Ok(_) => {
                    let mut buf = self.parse(&buf)?;
                    while buf.len() < len {
                        buf.push(self.read_u8().await?);
                    }
                    Ok(buf)
                }
                Err(err) => Err(Box::new(err)),
            }
        } else {
            return Err(Box::new(ConnectionError::ConnectionLost));
        }
    }

    fn disconnect(&mut self) -> TermComResult<()> {
        // self.tcp_stream.shutdown(std::net::Shutdown::Both)
        Ok(())
    }

    async fn send<'a>(&mut self, buf: &'a [u8]) -> TermComResult<usize> {
        if self.use_raw_transfer {
            return if let Some(stream) = self.tcp_stream.as_mut() {
                stream.write_all(buf).await?;
                Ok(buf.len())
            } else {
                Err(Box::new(ConnectionError::ConnectionLost))
            };
        }

        let mut data = Vec::with_capacity(buf.len());
        for b in buf {
            if *b == telnet_cmd::Iac {
                data.extend_from_slice(&[telnet_cmd::Iac, telnet_cmd::Iac]);
            } else {
                data.push(*b);
            }
        }
        if let Some(stream) = self.tcp_stream.as_mut() {
            stream.write_all(&data).await?;
            Ok(buf.len())
        } else {
            Err(Box::new(ConnectionError::ConnectionLost))
        }
    }
}
