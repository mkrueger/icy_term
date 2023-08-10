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

    use super::TelnetOption;

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

    pub fn make_cmd_opt(byte: u8, opt: TelnetOption) -> [u8; 3] {
        [Iac, byte, opt as u8]
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
}

/**
<http://www.iana.org/assignments/telnet-options/telnet-options.xhtml>
*/
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TelnetOption {
    /// https://www.rfc-editor.org/rfc/rfc856
    TransmitBinary = 0x00,
    /// https://www.rfc-editor.org/rfc/rfc857
    Echo = 0x01,
    /// ???
    Reconnection = 0x02,
    /// https://www.rfc-editor.org/rfc/rfc858
    SuppressGoAhead = 0x03,
    /// https://www.rfc-editor.org/rfc/rfc859
    Status = 0x05,
    /// https://www.rfc-editor.org/rfc/rfc860
    TimingMark = 0x06,
    /// https://www.rfc-editor.org/rfc/rfc726.html
    RemoteControlledTransAndEcho = 0x07,
    /// ???
    OutputLineWidth = 0x08,
    /// ???
    OutputPageSize = 0x09,
    ///https://www.rfc-editor.org/rfc/RFC652
    OutputCarriageReturnDisposition = 10,
    ///https://www.rfc-editor.org/rfc/RFC653
    OutputHorizontalTabStops = 11,
    ///https://www.rfc-editor.org/rfc/RFC654
    OutputHorizontalTabDisposition = 12,
    ///https://www.rfc-editor.org/rfc/RFC655
    OutputFormfeedDisposition = 13,
    ///https://www.rfc-editor.org/rfc/RFC656
    OutputVerticalTabstops = 14,
    ///https://www.rfc-editor.org/rfc/RFC657
    OutputVerticalTabDisposition = 15,
    ///https://www.rfc-editor.org/rfc/RFC658
    OutputLinefeedDisposition = 16,
    ///https://www.rfc-editor.org/rfc/RFC698
    ExtendedASCII = 17,
    ///https://www.rfc-editor.org/rfc/RFC727
    Logout = 18,
    ///https://www.rfc-editor.org/rfc/RFC735
    ByteMacro = 19,
    ///https://www.rfc-editor.org/rfc/RFC1043][RFC732
    DataEntryTerminal = 20,
    ///https://www.rfc-editor.org/rfc/RFC736][RFC734
    SupDup = 21,
    ///https://www.rfc-editor.org/rfc/RFC749
    SupDupOutput = 22,
    ///https://www.rfc-editor.org/rfc/RFC779
    SendLocation = 23,
    /// https://www.rfc-editor.org/rfc/rfc1091
    TerminalType = 24,
    /// https://www.rfc-editor.org/rfc/rfc885
    EndOfRecord = 25,
    /// https://www.rfc-editor.org/rfc/rfc1073
    NegotiateAboutWindowSize = 31,
    /// https://www.rfc-editor.org/rfc/rfc1079
    TerminalSpeed = 32,
    /// https://www.rfc-editor.org/rfc/rfc1372
    ToggleFlowControl = 33,
    /// https://www.rfc-editor.org/rfc/rfc1184
    LineMode = 34,
    /// https://www.rfc-editor.org/rfc/rfc1096
    XDisplayLocation = 35,
    /// https://www.rfc-editor.org/rfc/rfc1408
    EnvironmentOption = 36,
    /// https://www.rfc-editor.org/rfc/rfc2941
    Authentication = 37,
    /// https://www.rfc-editor.org/rfc/rfc2946
    Encrypt = 38,
    /// https://www.rfc-editor.org/rfc/rfc1572
    NewEnviron = 39,
    ///https://www.rfc-editor.org/rfc/RFC2355
    TN3270E = 40,
    ///https://www.rfc-editor.org/rfc/Rob_Earhart
    XAuth = 41,
    ///https://www.rfc-editor.org/rfc/RFC2066
    CharSet = 42,
    ///https://www.rfc-editor.org/rfc/Robert_Barnes
    TelnetRemoteSerialPortRSP = 43,
    ///https://www.rfc-editor.org/rfc/RFC2217
    ComPortControlOption = 44,
    ///https://www.rfc-editor.org/rfc/Wirt_Atmar
    TelnetSuppressLocalEcho = 45,
    ///https://www.rfc-editor.org/rfc/Michael_Boe
    TelnetStartTLS = 46,
    ///https://www.rfc-editor.org/rfc/RFC2840
    Kermit = 47,
    ///https://www.rfc-editor.org/rfc/David_Croft
    SendURL = 48,
    ///https://www.rfc-editor.org/rfc/Jeffrey_Altman
    ForwardX = 49,
    // 50-137 	Unassigned
    TelOptPragmaLogon = 138,
    ///https://www.rfc-editor.org/rfc/Steve_McGregory
    TelOptSSPILogon = 139,
    ///https://www.rfc-editor.org/rfc/Steve_McGregory
    TelOptPragmaHeartbeat = 140,
    ///https://www.rfc-editor.org/rfc/Steve_McGregory
    // 141-254 	Unassigned
    /// https://www.rfc-editor.org/rfc/rfc861
    ExtendedOptionsList = 0xFF,
}

#[allow(dead_code)]
impl TelnetOption {
    pub fn get(byte: u8) -> TermComResult<TelnetOption> {
        let cmd = match byte {
            0 => TelnetOption::TransmitBinary,
            1 => TelnetOption::Echo,
            2 => TelnetOption::Reconnection,
            3 => TelnetOption::SuppressGoAhead,
            5 => TelnetOption::Status,
            6 => TelnetOption::TimingMark,
            7 => TelnetOption::RemoteControlledTransAndEcho,
            8 => TelnetOption::OutputLineWidth,
            9 => TelnetOption::OutputPageSize,
            10 => TelnetOption::OutputCarriageReturnDisposition,
            11 => TelnetOption::OutputHorizontalTabStops,
            12 => TelnetOption::OutputHorizontalTabDisposition,
            13 => TelnetOption::OutputFormfeedDisposition,
            14 => TelnetOption::OutputVerticalTabstops,
            15 => TelnetOption::OutputVerticalTabDisposition,
            16 => TelnetOption::OutputLinefeedDisposition,
            17 => TelnetOption::ExtendedASCII,
            18 => TelnetOption::Logout,
            19 => TelnetOption::ByteMacro,
            20 => TelnetOption::DataEntryTerminal,
            21 => TelnetOption::SupDup,
            22 => TelnetOption::SupDupOutput,
            23 => TelnetOption::SendLocation,
            24 => TelnetOption::TerminalType,
            25 => TelnetOption::EndOfRecord,
            31 => TelnetOption::NegotiateAboutWindowSize,
            32 => TelnetOption::TerminalSpeed,
            33 => TelnetOption::ToggleFlowControl,
            34 => TelnetOption::LineMode,
            35 => TelnetOption::XDisplayLocation,
            36 => TelnetOption::EnvironmentOption,
            37 => TelnetOption::Authentication,
            38 => TelnetOption::Encrypt,
            39 => TelnetOption::NewEnviron,
            40 => TelnetOption::TN3270E,
            41 => TelnetOption::XAuth,
            42 => TelnetOption::CharSet,
            43 => TelnetOption::TelnetRemoteSerialPortRSP,
            44 => TelnetOption::ComPortControlOption,
            45 => TelnetOption::TelnetSuppressLocalEcho,
            46 => TelnetOption::TelnetStartTLS,
            47 => TelnetOption::Kermit,
            48 => TelnetOption::SendURL,
            49 => TelnetOption::ForwardX,
            // unassigned
            138 => TelnetOption::TelOptPragmaLogon,
            139 => TelnetOption::TelOptSSPILogon,
            140 => TelnetOption::TelOptPragmaHeartbeat,
            // unassigned
            255 => TelnetOption::ExtendedOptionsList,
            _ => {
                return Err(Box::new(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("unknown option: {byte}/x{byte:02X}"),
                )));
            }
        };
        Ok(cmd)
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
        }
    }

    fn parse(&mut self, data: &[u8]) -> TermComResult<Vec<u8>> {
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
                            if let ParserState::SubCommand(cmd) = self.state {
                                if cmd == TelnetOption::TerminalType as i32 {
                                    let mut buf: Vec<u8> = vec![
                                        telnet_cmd::Iac,
                                        telnet_cmd::SB,
                                        TelnetOption::TerminalType as u8,
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

                                    println!("Sending terminal type: {:?}", buf);

                                    if let Some(stream) = self.tcp_stream.as_mut() {
                                        stream.try_write(&buf)?;
                                    } else {
                                        return Err(Box::new(ConnectionError::ConnectionLost));
                                    }
                                }
                            }
                        }
                        24 => {
                            // Ternminal type
                            self.state = ParserState::SubCommand(TelnetOption::TerminalType as i32);
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
                    cmd => {
                        eprintln!("unsupported IAC: {cmd:?}");
                        self.state = ParserState::Data;
                    }
                },
                ParserState::Will => {
                    self.state = ParserState::Data;
                    let opt = TelnetOption::get(*b)?;
                    if let Some(stream) = self.tcp_stream.as_mut() {
                        if let TelnetOption::TransmitBinary = opt {
                            stream.try_write(&telnet_cmd::make_cmd_opt(
                                telnet_cmd::DO,
                                TelnetOption::TransmitBinary,
                            ))?;
                        } else if let TelnetOption::Echo = opt {
                            stream.try_write(&telnet_cmd::make_cmd_opt(
                                telnet_cmd::DO,
                                TelnetOption::Echo,
                            ))?;
                        } else {
                            eprintln!("unsupported will option {opt:?}");
                            stream.try_write(&telnet_cmd::make_cmd_opt(telnet_cmd::Dont, opt))?;
                        }
                    } else {
                        return Err(Box::new(ConnectionError::ConnectionLost));
                    }
                }
                ParserState::Wont => {
                    let opt = TelnetOption::get(*b)?;
                    eprintln!("Won't {opt:?}");
                    self.state = ParserState::Data;
                }
                ParserState::Do => {
                    self.state = ParserState::Data;
                    let opt = TelnetOption::get(*b)?;
                    if let Some(stream) = self.tcp_stream.as_mut() {
                        match opt {
                            TelnetOption::TransmitBinary => {
                                stream.try_write(&telnet_cmd::make_cmd_opt(
                                    telnet_cmd::Will,
                                    TelnetOption::TransmitBinary,
                                ))?;
                            }
                            TelnetOption::TerminalType => {
                                stream.try_write(&telnet_cmd::make_cmd_opt(
                                    telnet_cmd::Will,
                                    TelnetOption::TerminalType,
                                ))?;
                            }
                            TelnetOption::NegotiateAboutWindowSize => {
                                // NAWS: send our current window size
                                let mut buf: Vec<u8> = telnet_cmd::make_cmd_opt(
                                    telnet_cmd::SB,
                                    TelnetOption::NegotiateAboutWindowSize,
                                )
                                .to_vec();
                                buf.extend(self.window_size.width.to_be_bytes());
                                buf.extend(self.window_size.height.to_be_bytes());
                                buf.push(telnet_cmd::SE as u8);

                                stream.try_write(&buf)?;
                            }
                            _ => {
                                eprintln!("unsupported do option {opt:?}");
                                stream
                                    .try_write(&telnet_cmd::make_cmd_opt(telnet_cmd::Wont, opt))?;
                            }
                        }
                    } else {
                        return Err(Box::new(ConnectionError::ConnectionLost));
                    }
                }
                ParserState::Dont => {
                    let opt = TelnetOption::get(*b)?;
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
        let mut data = Vec::with_capacity(buf.len());
        for b in buf {
            if *b == telnet_cmd::Iac {
                data.extend_from_slice(&[telnet_cmd::Iac, telnet_cmd::Iac]);
            } else {
                data.push(*b);
            }
        }
        if let Some(stream) = self.tcp_stream.as_mut() {
            match stream.write(&data).await {
                Ok(bytes) => Ok(bytes),
                Err(error) => Err(Box::new(error)),
            }
        } else {
            Err(Box::new(ConnectionError::ConnectionLost))
        }
    }
}
