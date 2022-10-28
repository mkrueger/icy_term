use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::net::{ToSocketAddrs};
use iced::keyboard::{KeyCode, Modifiers};
use iced::widget::{Canvas, column, row, button, text, pick_list};
use iced::{executor, subscription, Event, keyboard};
use iced::{
    Application, Command, Element, Length, 
    Subscription, Theme,
};
use iced::widget::{
     text_input, vertical_rule
};
use iced::{Alignment};
use rfd::FileDialog;

use crate::input_conversion::UNICODE_TO_CP437;
use crate::model::{DEFAULT_FONT_NAME, BitFont};
use crate::{VERSION, iemsi};
use crate::address::{Address, start_read_book, READ_ADDRESSES};
use crate::com::{Com, TelnetCom};
use crate::iemsi::{IEmsi, EmsiICI};
use crate::protocol::{Xmodem, Zmodem, Ymodem, Protocol, ProtocolType, FileDescriptor};

use super::BufferView;
use super::screen_modes::{DEFAULT_MODES, ScreenMode};

enum MainWindowMode {
    Default,
    ShowPhonebook,
    SelectProtocol(bool),
    FileTransfer(ProtocolType, bool)
}

struct Options {
    connect_timeout: Duration
}

impl Options {
    pub fn new() -> Self {
        Options {
            connect_timeout: Duration::from_secs(10)
        }
    }
}

pub struct MainWindow<T: Com> {
    buffer_view: BufferView,
    telnet: Option<T>,
    trigger: bool,
    mode: MainWindowMode,
    pub addresses: Vec<Address>,
    cur_addr: usize,
    logged_in: bool,
    log_file: Vec<String>,
    options: Options,
    iemsi: Option<IEmsi>,
    connection_time: SystemTime,
    font: Option<String>,
    screen_mode: Option<ScreenMode>,
    // protocols
    xmodem: Xmodem,
    ymodem: Ymodem,
    zmodem: Zmodem
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    InitiateFileTransfer(bool),
    SendLogin,
    ShowPhonebook,
    Back,
    Hangup,
    Edit,
    KeyPressed(char),
    KeyCode(KeyCode, Modifiers),
    CallBBS(usize),
    QuickConnectChanged(String),
    FontSelected(String),
    ScreenModeSelected(ScreenMode),
    SelectProtocol(ProtocolType, bool)
}

static KEY_MAP: &[(KeyCode, &[u8])] = &[
    (KeyCode::Home, "\x1b[1~".as_bytes()),
    (KeyCode::Insert, "\x1b[2~".as_bytes()),
    (KeyCode::Delete, "\x1b[3~".as_bytes()),
    (KeyCode::End, "\x1b[4~".as_bytes()),
    (KeyCode::PageUp, "\x1b[5~".as_bytes()),
    (KeyCode::PageDown, "\x1b[6~".as_bytes()),
    (KeyCode::F1, "\x1b[11~".as_bytes()),
    (KeyCode::F2, "\x1b[12~".as_bytes()),
    (KeyCode::F3, "\x1b[13~".as_bytes()),
    (KeyCode::F4, "\x1b[14~".as_bytes()),
    (KeyCode::F5, "\x1b[15~".as_bytes()),
    (KeyCode::F6, "\x1b[17~".as_bytes()),
    (KeyCode::F7, "\x1b[18~".as_bytes()),
    (KeyCode::F8, "\x1b[19~".as_bytes()),
    (KeyCode::F9, "\x1b[20~".as_bytes()),
    (KeyCode::F10, "\x1b[21~".as_bytes()),
    (KeyCode::F11, "\x1b[23~".as_bytes()),
    (KeyCode::F12, "\x1b[24~".as_bytes()),
    (KeyCode::Up, "\x1b[A".as_bytes()),
    (KeyCode::Down, "\x1b[B".as_bytes()),
    (KeyCode::Right, "\x1b[C".as_bytes()),
    (KeyCode::Left, "\x1b[D".as_bytes())
];

impl MainWindow<TelnetCom> 
{
    pub fn update_state(&mut self) -> io::Result<()>
    {
        match &mut self.telnet {
            None => Ok(()),
            Some(telnet) => {
                let mut do_update = false;
                while telnet.is_data_available()? {
                    let ch = telnet.read_char_nonblocking()?;
            
                    if let Some(iemsi) = &mut self.iemsi {
                        iemsi.push_char(ch)?;
                        if iemsi.irq_requested {
                            iemsi.irq_requested = false;
                            self.log_file.push("Starting IEMSI negotiation…".to_string());
                            let mut data = EmsiICI::new();
                            let adr = self.addresses.get(self.cur_addr).unwrap();
                            data.name = adr.user_name.clone();
                            data.password = adr.password.clone();
                            telnet.write(&data.encode().unwrap())?;
                            self.logged_in = true;
                        } else if let Some(isi) = &iemsi.isi  {
                            self.log_file.push("Receiving valid IEMSI server info…".to_string());
                            self.log_file.push(format!("Name:{} Location:{} Operator:{} Notice:{} System:{}", isi.name, isi.location, isi.operator, isi.notice, isi.id));
                            telnet.write(iemsi::EMSI_2ACK)?;
                            self.logged_in = true;
                            self.iemsi = None;
                        } else if iemsi.got_invavid_isi  {
                            iemsi.got_invavid_isi = false;
                            self.log_file.push("Got invalid IEMSI server info…".to_string());
                            telnet.write(iemsi::EMSI_2ACK)?;
                            self.logged_in = true;
                            self.iemsi = None;
                        } else if iemsi.nak_requested && self.logged_in {
                            iemsi.nak_requested = false;
                            if iemsi.retries < 2  {
                                self.log_file.push("IEMSI retry…".to_string());
                                let mut data = EmsiICI::new();
                                let adr = self.addresses.get(self.cur_addr).unwrap();
                                data.name = adr.user_name.clone();
                                data.password = adr.password.clone();
                                telnet.write(&data.encode().unwrap())?;
                                iemsi.retries += 1;
                            } else  {
                                self.log_file.push("IEMSI aborted…".to_string());
                                telnet.write(iemsi::EMSI_IIR)?;
                                self.iemsi = None;
                            }
                        }
                    }
        
                    self.buffer_view.print_char(telnet, ch)?;
                    do_update = true;
                }
                if do_update {
                    self.buffer_view.cache.clear();
                }
                Ok(())
            }
        }
    }

    pub fn get_screen_mode(&self) -> ScreenMode
    {
        if let Some(mode) = self.screen_mode {
            return mode;
        }

        return ScreenMode::DOS(80, 25);
    }

    pub fn get_font_name(&self) -> String
    {
        if let Some(font) = &self.font {
            return font.clone();
        }

        return DEFAULT_FONT_NAME.to_string();
    }
    
    pub fn print_log(&mut self, str: String)
    {
        self.log_file.push(str);
    }

    pub fn print_result<T>(&mut self, result: &io::Result<T>)
    {
        if let Err(error) = result {
            self.log_file.push(format!("{}", error));
        }
    }

    pub fn set_font(&mut self, font: &String)
    {
        if font != &self.get_font_name() { 
            self.font = Some(font.clone());
            self.buffer_view.buf.font = BitFont::from_name(&self.get_font_name()).unwrap();
            self.buffer_view.cache.clear();
        }
    }

    pub fn set_screen_mode(&mut self, mode: &ScreenMode)
    {
        if mode != &self.get_screen_mode() { 
            self.screen_mode = Some(*mode);
            self.get_screen_mode().set_mode(&mut self.font, &mut self.buffer_view.buf);
            self.buffer_view.buf.font = BitFont::from_name(&self.get_font_name()).unwrap();
            self.buffer_view.cache.clear();
        }
    }
}

impl Application for MainWindow<TelnetCom> {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn title(&self) -> String {
        format!("iCY TERM {}", VERSION)
    }

    fn new(_flags: ()) ->  (Self, Command<Message>) {
       let mut view =  MainWindow {
            buffer_view: BufferView::new(),
            telnet:None,
            trigger: true,
            mode: MainWindowMode::Default,
            addresses: start_read_book(),
            cur_addr: 0,
            logged_in: false,
            connection_time: SystemTime::now(),
            log_file: Vec::new(),
            options: Options::new(),
            iemsi: None,
            xmodem: Xmodem::new(),
            ymodem: Ymodem::new(),
            zmodem: Zmodem::new(),
            font: Some(DEFAULT_FONT_NAME.to_string()),
            screen_mode: None
        };
        view.buffer_view.buf.clear();
        (view, Command::none())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.trigger = !self.trigger;

        if unsafe { READ_ADDRESSES } {
            unsafe { READ_ADDRESSES = false; } 
            self.addresses = Address::read_phone_book();
        }

        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let in_ms = since_the_epoch.as_millis();
        
        if in_ms - self.buffer_view.last_blink > 550 {
            self.buffer_view.blink = !self.buffer_view.blink;
            self.buffer_view.last_blink = in_ms;
        }

        match self.mode {
            MainWindowMode::Default => {
                match message {
                    Message::InitiateFileTransfer(download)=> {
                        self.mode = MainWindowMode::SelectProtocol(download);
                    },
                    Message::SendLogin => {
                        if let Some(telnet) = &mut self.telnet {
                            let adr = self.addresses.get(self.cur_addr).unwrap();
                            if let Err(err) = telnet.write([adr.user_name.as_bytes(), b"\r", adr.password.as_bytes(), b"\r"].concat().as_slice()) {
                                self.print_log(format!("Error sending login: {}", err));
                            }
                            self.logged_in = true;
                        }
                    }
                    Message::ShowPhonebook => {
                        self.mode = MainWindowMode::ShowPhonebook;
                    },
                    Message::Hangup => {
                        self.telnet = None;
                        self.print_log(format!("Disconnected."));

                    },
                    Message::Tick => { 
                        let state = self.update_state(); 

                        if let Err(s) = state {
                            self.print_log(format!("Error: {:?}", s));
                        }
                    },
                    Message::KeyPressed(ch) => {
                        if let Some(telnet) = &mut self.telnet {
                            let data =  [if let Some(c) =  UNICODE_TO_CP437.get(&ch) { *c } else { ch as u8 }];
                            let state = telnet.write(&data);
                            if let Err(s) = state {
                                self.print_log(format!("Error: {:?}", s));
                                self.telnet = None;
                            }
                        }
                    },
                    Message::KeyCode(code, _modifier) => {
                        if let Some(telnet) = &mut self.telnet {
                            for (k, m) in KEY_MAP {
                                if *k == code {
                                    let state = telnet.write(m);
                                    if let Err(s) = state {
                                        self.print_log(format!("Error: {:?}", s));
                                        self.telnet = None;
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    Message::FontSelected(font) => {
                        self.set_font(&font);
                    }
                    Message::ScreenModeSelected(mode) => {
                        self.set_screen_mode(&mode);
                    }
                    _ => {}
                }

            },
            MainWindowMode::ShowPhonebook => {
                text_input::focus::<Message>(super::INPUT_ID.clone());
                match message {
                    Message::ShowPhonebook => {
                        self.mode = MainWindowMode::ShowPhonebook
                    },
                    Message::Back => {
                        self.mode = MainWindowMode::Default
                    },
                    Message::Edit => {
                        if let Some(phonebook) = Address::get_phonebook_file() {
                           if let Err(err) =  open::that(phonebook) {
                               self.print_log(format!("Error open phonebook file: {:?}", err));
                               self.mode = MainWindowMode::Default
                           }
                        }
                    },
                    
                    Message::CallBBS(i) => {
                        self.mode = MainWindowMode::Default;
                        let mut adr = self.addresses[i].address.clone();
                        if !adr.contains(":") {
                            adr.push_str(":23");
                        }
                        self.print_log(format!("Connect to {}…", adr));
                        let mut socket_addr = adr.to_socket_addrs();
                        match &mut socket_addr {
                            Ok(socket_addr) => {
                                if let Some(addr) = &socket_addr.next() {
                                    let t = TelnetCom::connect(addr, self.options.connect_timeout);
                                    match t {
                                        Ok(t) => {
                                            self.logged_in = false;
                                            self.telnet = Some(t);
                                            self.cur_addr = i;
                                            self.iemsi = Some(IEmsi::new());
                                            self.connection_time = SystemTime::now();
                                            let adr = self.addresses[i].clone();
                                            if let Some(mode) = &adr.screen_mode {
                                                self.set_screen_mode(mode);
                                            }
                                            if let Some(font) = &adr.font_name {
                                                self.set_font(font);
                                            }
                                        },
                                        Err(e) => {
                                            self.print_log(format!("Error: {:?}", e));
                                        }
                                    }
                                }
                            }
                            Err(error) => {
                                self.print_log(format!("Socket error: {:?}", error));
                            }
                        }
                    },

                    Message::QuickConnectChanged(addr) => {
                        self.addresses[0].address = addr
                    }
                    _ => {}
                }
            },
            MainWindowMode::SelectProtocol(_) => {
                match message {
                    Message::Back => {
                        self.mode = MainWindowMode::Default
                    }
                    Message::SelectProtocol(protocol_type, download) => {
                        self.mode = MainWindowMode::Default;
                        if !download {
                            let files = FileDialog::new()
                                .pick_files();
                                if let Some(path) = files {
                                    let fd = FileDescriptor::from_paths(&path);
                                    self.print_result(&fd);
                                    if let Ok(files) =  fd {
                                        let r = match protocol_type {
                                            ProtocolType::ZModem => {
                                                self.zmodem.initiate_send(self.telnet.as_mut().unwrap(), files)
                                            },
                                            ProtocolType::ZedZap => {
                                                self.zmodem.initiate_send(self.telnet.as_mut().unwrap(), files)
                                            },
                                            ProtocolType::XModem => {
                                                self.xmodem.initiate_send(self.telnet.as_mut().unwrap(), files)
                                            },
                                            ProtocolType::YModem => {
                                                self.ymodem.initiate_send(self.telnet.as_mut().unwrap(), files)
                                            },
                                            ProtocolType::YModemG => {
                                                self.ymodem.initiate_send(self.telnet.as_mut().unwrap(), files)
                                            }
                                        };
                                        self.print_result(&r);
                                        if r.is_ok() {
                                            self.mode = MainWindowMode::FileTransfer(protocol_type, download);
                                        }
                                    }
                                } 
                        } else {
                            let r = match protocol_type {
                                ProtocolType::ZModem => {
                                    self.zmodem.initiate_recv(self.telnet.as_mut().unwrap())
                                },
                                ProtocolType::ZedZap => {
                                    self.zmodem.initiate_recv(self.telnet.as_mut().unwrap())
                                },
                                ProtocolType::XModem => {
                                    self.xmodem.initiate_recv(self.telnet.as_mut().unwrap())
                                },
                                ProtocolType::YModem => {
                                    self.ymodem.initiate_recv(self.telnet.as_mut().unwrap())
                                },
                                ProtocolType::YModemG => {
                                    self.ymodem.initiate_recv(self.telnet.as_mut().unwrap())
                                }
                            };
                            self.print_result(&r);
                            if r.is_ok() {
                                self.mode = MainWindowMode::FileTransfer(protocol_type, download);
                            }
                        }
                    }
                    _ => { }
                }
            }
            MainWindowMode::FileTransfer(protocol_type, _download) => {

                match message {
                    Message::Tick => { 
                        if let Some(com) = &mut self.telnet {

                            let r = match protocol_type {
                                ProtocolType::ZModem | ProtocolType::ZedZap => {
                                    self.zmodem.update(com)
                                },
                                ProtocolType::XModem => {
                                    self.xmodem.update(com)
                                },
                                ProtocolType::YModem | ProtocolType::YModemG => {
                                    self.ymodem.update(com)
                                },
                            };
                            self.print_result(&r);
                        }

                        if !self.zmodem.is_active() && !self.xmodem.is_active() && !self.ymodem.is_active() {
                            self.mode = MainWindowMode::Default;
                        }

                    },
                    _ => {}
                }
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        
        let s = subscription::events_with(|event, status| match (event, status) {
            (
                Event::Keyboard(keyboard::Event::CharacterReceived(ch)),
                iced::event::Status::Ignored,
            ) => Some(Message::KeyPressed(ch)),
            (
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                    ..
                }),
                iced::event::Status::Ignored,
            ) => Some(Message::KeyCode(key_code, modifiers)),
            _ => None,
        });

        let t = iced::time::every(std::time::Duration::from_millis(10))
            .map(|_| Message::Tick);

        Subscription::<Message>::batch([s, t])
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        
        match self.mode {
            MainWindowMode::Default => {
                let c = Canvas::new(&self.buffer_view)
                    .width(Length::Fill)
                    .height(Length::Fill);
                
                let log_info = if self.log_file.len() == 0  { text("Ready.")} else { text(&self.log_file[self.log_file.len() - 1])}.width(Length::Fill).into();
                let all_fonts = crate::model::_SUPPORTED_FONTS.map(|s| s.to_string()).to_vec();
                let font_pick_list = pick_list(
                    all_fonts,
                    Some(self.get_font_name()),
                    Message::FontSelected
                );

                let screen_mode_pick_list: iced_native::widget::pick_list::PickList<'_, ScreenMode, Message, iced::Renderer> = pick_list(
                    DEFAULT_MODES.to_vec(),
                    Some(self.get_screen_mode()),
                    Message::ScreenModeSelected
                );

                column(vec![
                    if !self.logged_in && self.telnet.is_some() && self.addresses[self.cur_addr].user_name.len() > 0 {
                        row![
                            button("Phonebook")
                                .on_press(Message::ShowPhonebook),
                            button("Upload")
                                .on_press(Message::InitiateFileTransfer(false)),
                            button("Download")
                                .on_press(Message::InitiateFileTransfer(true)),
                            button("Send login")
                                .on_press(Message::SendLogin),
                                button("Hangup")
                                .on_press(Message::Hangup)
                        ]
                    } else {
                        if  self.telnet.is_some()  {
                            row![
                                button("Phonebook")
                                    .on_press(Message::ShowPhonebook),
                                button("Upload")
                                    .on_press(Message::InitiateFileTransfer(false)),
                                button("Download")
                                    .on_press(Message::InitiateFileTransfer(true)),
                                button("Hangup")                            
                                    .on_press(Message::Hangup) 
                            ]
                        } else {
                            row![
                                button("Phonebook")
                                    .on_press(Message::ShowPhonebook),
                                button("Upload")
                                    .on_press(Message::InitiateFileTransfer(false)),
                                button("Download")
                                    .on_press(Message::InitiateFileTransfer(true)),
                            ]
                        }
                    }.padding(10).spacing(20).into(),
                    c.into(),
                    if self.telnet.is_none() {
                        row(vec![
                            log_info,
                            vertical_rule(10).into(),
                            text("Offline").into(),
                            vertical_rule(10).into(),
                            font_pick_list.into(),
                            screen_mode_pick_list.into(),
                        ])
                    } else {
                        let d = SystemTime::now().duration_since(self.connection_time).unwrap();
                        let sec     = d.as_secs();
                        let minutes = sec / 60;
                        let hours   = minutes  / 60;
                        let cur = &self.addresses[self.cur_addr];

                        row(vec![
                            log_info,
                            vertical_rule(10).into(),
                            text(if cur.system_name.len() > 0 { &cur.system_name } else { &cur.address }).into(),
                            vertical_rule(10).into(),
                            text(format!("Connected {:02}:{:02}:{:02}", hours, minutes % 60, sec % 60)).into(),
                            vertical_rule(10).into(),
                            font_pick_list.into(),
                            screen_mode_pick_list.into(),
                        ])
                    }
                    .padding(8)
                    .spacing(20)
                    .height(Length::Units(40))
                    .align_items(Alignment::Start)
                    .into()
                ]).spacing(8)
                .into()
            },
            MainWindowMode::ShowPhonebook => {   
                super::view_phonebook(self)            
            },
            MainWindowMode::SelectProtocol(download) => {   
                super::view_protocol_selector(download)
            }
            MainWindowMode::FileTransfer(protocol_type, download) => {
                match protocol_type {
                    ProtocolType::ZModem => {
                        super::view_file_transfer(&self.zmodem, download)
                    },
                    ProtocolType::ZedZap => {
                        super::view_file_transfer(&self.zmodem, download)
                    },
                    ProtocolType::XModem => {
                        super::view_file_transfer(&self.xmodem, download)
                    },
                    ProtocolType::YModem => {
                        super::view_file_transfer(&self.ymodem, download)
                    },
                    ProtocolType::YModemG => {
                        super::view_file_transfer(&self.ymodem, download)
                    }
                }
            }
        }
    }
}

