use std::{ env};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::net::{ToSocketAddrs};
use clipboard::{ClipboardProvider, ClipboardContext};
use iced::keyboard::{KeyCode};
use iced::mouse::ScrollDelta;
use iced::widget::{Canvas, column, button, text, Row};
use iced::{executor, subscription, Event, keyboard, mouse, Alignment};
use iced::{
    Application, Command, Element, Length, 
    Subscription, Theme,
};
use iced::widget::{
     text_input
};
use icy_engine::{DEFAULT_FONT_NAME, BitFont};
use rfd::FileDialog;

use crate::auto_file_transfer::AutoFileTransfer;
use crate::auto_login::AutoLogin;
use crate::{VERSION};
use crate::address::{Address, start_read_book, READ_ADDRESSES, store_phone_book};
use crate::com::{Com, TelnetCom};
use crate::protocol::{ Protocol, FileDescriptor, TransferState};

use super::{BufferView, Message, ANSI_KEY_MAP, C64_KEY_MAP, ATASCII_KEY_MAP, CTRL_MOD, SHIFT_MOD};
use super::screen_modes::{ ScreenMode};

enum MainWindowMode {
    Default,
    ShowPhonebook,
    SelectProtocol(bool),
    FileTransfer(bool),
    EditBBS(usize)
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

pub struct MainWindow {
    pub buffer_view: BufferView,
    com: Option<Box<dyn Com>>,
    trigger: bool,
    mode: MainWindowMode,
    pub addresses: Vec<Address>,
    pub handled_char: bool,
    edit_bbs: Address,
    cur_addr: usize,
    log_file: Vec<String>,
    options: Options,
    connection_time: SystemTime,
    font: Option<String>,
    screen_mode: Option<ScreenMode>,
    auto_login: AutoLogin,
    auto_file_transfer: AutoFileTransfer,
    // protocols
    current_protocol: Option<(Box<dyn Protocol>, TransferState)>,
    is_alt_pressed: bool
}

impl MainWindow
{
    pub fn update_state(&mut self) -> Result<(), Box<dyn std::error::Error>>
    {
        match &mut self.com {
            None => Ok(()),
            Some(com) => {
                self.auto_login.disabled |= self.is_alt_pressed;
                if let Some(adr) = self.addresses.get(self.cur_addr) {
                    if let Err(err) = self.auto_login.run_autologin(com, adr) {
                        eprintln!("{}", err);
                        self.log_file.push(format!("{}", err));
                    }
                }
                let mut do_update = false;
                while com.is_data_available()? {
                    let ch = com.read_char_nonblocking()?;
                    if let Some(adr) = self.addresses.get(self.cur_addr) {
                        if let Err(err) = self.auto_login.try_login(com, adr, ch) {
                            eprintln!("{}", err);
                            self.log_file.push(format!("{}", err));
                        }
                    }


                    self.buffer_view.print_char(Some(com.as_mut()), ch)?;
                    do_update = true;
                    if let Some((protocol_type, download)) = self.auto_file_transfer.try_transfer(ch) {
//                        if !download {
//                            self.mode = MainWindowMode::SelectProtocol(download);
//                        } else {
                            self.initiate_file_transfer(protocol_type, download);
//                        }
                        return Ok(());
                    }
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

    pub fn print_result<'a, T>(&mut self, result: &Result<T, Box<dyn std::error::Error + 'a>>)
    {
        if let Err(error) = result {
            eprintln!("{}", error);
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
        self.screen_mode = Some(*mode);
        self.get_screen_mode().set_mode(&mut self.font, &mut self.buffer_view);
        self.buffer_view.buf.font = BitFont::from_name(&self.get_font_name()).unwrap();
        self.buffer_view.cache.clear();
    }

    pub fn output_char(&mut self, ch: char) 
    {
        let translated_char = self.buffer_view.buffer_parser.from_unicode(ch);
        if let Some(com) = &mut self.com {
            let state = com.write(&[translated_char as u8]);
            if let Err(err) = state {
                eprintln!("{}", err);
                self.print_log(format!("Error: {:?}", err));
                self.com = None;
            }
        } else {
            let r = self.buffer_view.print_char(None, translated_char as u8);
            self.print_result(&r);
            self.buffer_view.cache.clear();
        }
    }

    fn initiate_file_transfer(&mut self, protocol_type: crate::protocol::ProtocolType, download: bool) {
        self.mode = MainWindowMode::Default;
        if let Some(com) = self.com.as_mut() {
            if !download {
                let files = FileDialog::new()
                    .pick_files();
                if let Some(path) = files {
                    let fd = FileDescriptor::from_paths(&path);
                    if let Ok(files) =  fd {
                        let mut protocol = protocol_type.create();
                        match protocol.initiate_send(com, files) {
                            Ok(state) => {
                                self.mode = MainWindowMode::FileTransfer(download);
                                self.current_protocol = Some((protocol, state));
                            }
                            Err(error) => {
                                eprintln!("{}", error);
                                self.log_file.push(format!("{}", error));
                            }
                        }
                    } else {
                        self.print_result(&fd);
                    }
                }
            } else {
                let mut protocol = protocol_type.create();
                match protocol.initiate_recv(com) {
                    Ok(state) => {
                        self.mode = MainWindowMode::FileTransfer(download);
                        self.current_protocol = Some((protocol, state));
                    }
                    Err(error) => {
                        eprintln!("{}", error);
                        self.log_file.push(format!("{}", error));
                    }
                }
            }
        } else {
            self.print_log("Communication error.".to_string());
        }
    }
}

impl Application for MainWindow {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn title(&self) -> String {
        let str = if self.com.is_some() {
            let d = SystemTime::now().duration_since(self.connection_time).unwrap();
            let sec     = d.as_secs();
            let minutes = sec / 60;
            let hours   = minutes  / 60;
            let cur = &self.addresses[self.cur_addr];
            
            format!("Connected {:02}:{:02}:{:02} to {}", hours, minutes % 60, sec % 60, if cur.system_name.len() > 0 { &cur.system_name } else { &cur.address })
        } else { 
            "Offline".to_string()
        };
        format!("iCY TERM {} - {}", VERSION, str)
    }

    fn new(_flags: ()) ->  (Self, Command<Message>) {
       let mut view =  MainWindow {
            buffer_view: BufferView::new(),
            com:None,
            trigger: true,
            mode: MainWindowMode::Default,
            addresses: start_read_book(),
            edit_bbs: Address::new(),
            cur_addr: 0,
            connection_time: SystemTime::now(),
            log_file: Vec::new(),
            options: Options::new(),
            auto_login: AutoLogin::new(String::new()),
            auto_file_transfer: AutoFileTransfer::new(),
            font: Some(DEFAULT_FONT_NAME.to_string()),
            screen_mode: None,
            current_protocol: None,
            handled_char: false,
            is_alt_pressed: false
        };

       //  view.set_screen_mode(&ScreenMode::DOS(80, 50));
       /*let txt = b"";
       for b in txt {
           if let Err(err) = view.buffer_view.buffer_parser.print_char(&mut view.buffer_view.buf, &mut view.buffer_view.caret, *b) {
               eprintln!("{}", err);
           }
       }*/
      
        let args: Vec<String> = env::args().collect();
        if let Some(arg) = args.get(1) {
            println!("{}", arg);
            view.addresses[0].address = arg.clone();
            view.call_bbs(0);
        }

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
        
        match &message {
            Message::OpenURL(url) => {
                if let Err(err) = open::that(url) {
                    eprintln!("{}", err);
                }
            }
            _ => {}
        };

        match self.mode {
            MainWindowMode::Default => {
                match message {
                    Message::InitiateFileTransfer(download)=> {
                        self.mode = MainWindowMode::SelectProtocol(download);
                    },
                    Message::SendLogin => {
                        if let Some(com) = &mut self.com {
                            let adr = self.addresses.get(self.cur_addr).unwrap();
                            if let Err(err) = com.write([adr.user_name.as_bytes(), b"\r", adr.password.as_bytes(), b"\r"].concat().as_slice()) {
                                eprintln!("Error sending login: {}", err);
                                self.print_log(format!("Error sending login: {}", err));
                            }
                            self.auto_login.logged_in = true;
                        }
                    }
                    Message::ShowPhonebook => {
                        self.mode = MainWindowMode::ShowPhonebook;
                    },
                    Message::Hangup => {
                        self.com = None;
                        self.print_log(format!("Disconnected."));

                    },
                    Message::Tick => { 
                        let state = self.update_state(); 

                        if let Err(err) = state {
                            eprintln!("{}", err);
                            self.print_log(format!("Error: {:?}", err));
                        }
                    },
                    Message::CharacterReceived(ch) => {
                        if self.handled_char {
                            self.handled_char = false;
                        } else {
                            self.output_char(ch);
                        }
                    },
                    Message::KeyReleased(_, _) => {
                        self.handled_char = false;
                    }
                    Message::KeyPressed(code, modifier) => {
                        let mut code = code as u32;
                        if modifier.control() || modifier.command() {
                            code |= CTRL_MOD;
                        }
                        if modifier.shift() {
                            code |= SHIFT_MOD;
                        }
                        let map = match self.buffer_view.petscii {
                            super::BufferInputMode::CP437 => ANSI_KEY_MAP,
                            super::BufferInputMode::PETSCII => C64_KEY_MAP,
                            super::BufferInputMode::ATASCII => ATASCII_KEY_MAP,
                        }; 

                        if let Some(com) = &mut self.com {
                            for (k, m) in map {
                                if *k == code {
                                    self.handled_char = true;
                                    let state = com.write(m);
                                    if let Err(err) = state {
                                        eprintln!("{}", err);
                                        self.print_log(format!("Error: {:?}", err));
                                        self.com = None;
                                    }
                                    break;
                                }
                            }
                        } else {
                            for (k, m) in map {
                                if *k == code {
                                    self.handled_char = true;
                                    for ch in *m {
                                        let state = self.buffer_view.print_char(None, *ch);
                                        if let Err(s) = state {
                                            self.print_log(format!("Error: {:?}", s));
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    Message::AltKeyPressed(b) => self.is_alt_pressed = b,
                    Message::WheelScrolled(delta) => {
                        if let ScrollDelta::Lines { y, .. } = delta {
                            self.buffer_view.scroll(y as i32);
                            self.buffer_view.cache.clear();
                        }
                    }
                   /*  Message::FontSelected(font) => {
                        self.set_font(&font);
                    }
                    Message::ScreenModeSelected(mode) => {
                        self.set_screen_mode(&mode);
                    }*/
                    Message::Copy => { 
                        self.buffer_view.copy_to_clipboard();
                    }
                    Message::Paste => {
                        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
                        if let Ok(r) = ctx.get_contents() {
                            for c in r.chars() {
                                self.output_char(c);
                            }
                        }
                    }
                    Message::SetSelection(selection) => {
                        self.buffer_view.selection = selection;
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

                    Message::EditBBS(i) => {
                        self.edit_bbs = if i == 0 { Address::new() } else { self.addresses[i].clone() };
                        self.mode = MainWindowMode::EditBBS(i)
                    }
                    
                    Message::CallBBS(i) => {
                        self.call_bbs(i);
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
                        self.initiate_file_transfer(protocol_type, download);
                    }
                    _ => { }
                }
            }
            MainWindowMode::FileTransfer(_) => {
                match message {
                    Message::Tick => { 
                        if let Some(com) = self.com.as_mut() {
                            if let Some((protocol, state)) = &mut self.current_protocol {
                                match protocol.update(com, state) {
                                    Err(err) => { eprintln!("Err {}", err); }
                                    _ => {}
                                }
                               // self.print_result(&r);
                                if state.is_finished {
                                    for f in protocol.get_received_files() {
                                        f.save_file_in_downloads(state.recieve_state.as_mut().unwrap()).expect("error saving file.");
                                    }
                                    self.mode = MainWindowMode::Default;
                                    self.auto_file_transfer.reset();
                                }
                            }
                        }
                    },
                    Message::Back => {
                        self.current_protocol = None;
                        self.mode = MainWindowMode::Default;
                        self.auto_file_transfer.reset();
                    }
                    Message::CancelTransfer => {
                        if let Some(com) = &mut self.com {
                            
                            if let Some((protocol, state)) = &mut self.current_protocol {
                                if let Some(s) = &mut state.send_state {
                                    s.write("Send cancel.".to_string());
                                }
                                if let Some(s) = &mut state.recieve_state {
                                    s.write("Send cancel.".to_string());
                                }

                                if let Err(err) = protocol.cancel(com) {    
                                    if let Some(s) = &mut state.send_state {
                                        s.write(format!("Error while cancel {:?}", err));
                                    }
                                    if let Some(s) = &mut state.recieve_state {
                                        s.write(format!("Error while cancel {:?}", err));
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            MainWindowMode::EditBBS(_) => {
                text_input::focus::<Message>(super::INPUT_ID.clone());
                match message {
                    Message::Back => {
                        self.mode = MainWindowMode::ShowPhonebook;
                    },

                    Message::EditBbsSystemNameChanged(str) => self.edit_bbs.system_name = str,
                    Message::EditBbsAddressChanged(str) => self.edit_bbs.address = str,
                    Message::EditBbsUserNameChanged(str) => self.edit_bbs.user_name = str,
                    Message::EditBbsPasswordChanged(str) => self.edit_bbs.password = str,
                    Message::EditBbsCommentChanged(str) => self.edit_bbs.comment = str,
                    Message::EditBbsTerminalTypeSelected(terminal) => self.edit_bbs.terminal_type = terminal,
                    Message::EditBbsScreenModeSelected(screen_mode) => self.edit_bbs.screen_mode = Some(screen_mode),
                    Message::EditBbsAutoLoginChanged(str) => self.edit_bbs.auto_login = str,
                    Message::EditBbsConnectionType(connection_type) => self.edit_bbs.connection_type = connection_type,
                    Message::EditBbsSaveChanges(i) => {
                        if i == 0 { 
                            self.addresses.push(self.edit_bbs.clone());
                        } else {
                            self.addresses[i] = self.edit_bbs.clone();
                        }
                        self.print_result(&store_phone_book(&self.addresses));
                        self.mode = MainWindowMode::ShowPhonebook;
                    }
                    Message::EditBbsDeleteEntry(i) => {
                        if i > 0 { 
                            self.addresses.remove(i);
                        }
                        self.print_result(&store_phone_book(&self.addresses));
                        self.mode = MainWindowMode::ShowPhonebook;
                    }
                    _ => {}
                }
            }

        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        
        let s = subscription::events_with(|event, status| match (event, status) {
            (Event::Keyboard(keyboard::Event::CharacterReceived(ch)), iced::event::Status::Ignored) => Some(Message::CharacterReceived(ch)),
            (Event::Keyboard(keyboard::Event::KeyPressed {key_code, modifiers, ..}), iced::event::Status::Ignored) => Some(Message::KeyPressed(key_code, modifiers)),
            (Event::Keyboard(keyboard::Event::KeyReleased {key_code, modifiers, ..}), iced::event::Status::Ignored) => Some(Message::KeyReleased(key_code, modifiers)),
            (Event::Mouse(mouse::Event::WheelScrolled {delta, ..}), iced::event::Status::Ignored) => Some(Message::WheelScrolled(delta)),
            /*(Event::Window(ev), iced::event::Status::Ignored) => {
                println!("{:?}",ev );
                None
            },*/

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

               
                
                let mut title_row = Row::new()
                     .push(button("Phonebook")
                        .on_press(Message::ShowPhonebook));

                if self.com.is_some()  {
                    title_row = title_row.push(button("Upload")
                    .on_press(Message::InitiateFileTransfer(false)));
                    title_row = title_row.push(button("Download")
                    .on_press(Message::InitiateFileTransfer(true)));

                    if !self.auto_login.logged_in {
                        title_row = title_row.push(button("Send login")
                        .on_press(Message::SendLogin));
    
                    }

                    title_row = title_row.push(button("Hangup")
                    .on_press(Message::Hangup));

                }

                let log_info = if self.log_file.len() == 0  { text("")} else { text(&self.log_file[self.log_file.len() - 1])}
                .size(16)
                .width(Length::Fill);
                title_row = title_row.push(log_info);

                column(vec![
                    title_row.align_items(Alignment::Center) .spacing(8).padding(8).into(),
                    c.into()
                ])
                .into()
            }
            MainWindowMode::ShowPhonebook => {   
                super::view_phonebook(self)            
            }
            MainWindowMode::SelectProtocol(download) => {   
                super::view_protocol_selector(download)
            }
            MainWindowMode::EditBBS(i) => {
                super::view_edit_bbs(self, &self.edit_bbs, i)
            }
            MainWindowMode::FileTransfer(download) => {
                if let Some((_, state)) = &self.current_protocol {
                    super::view_file_transfer(state, download)
                } else {
                     text("invalid").into()
                }
            }
        }
    }
}

impl MainWindow {
    fn call_bbs(&mut self, i: usize) 
    {
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
                            self.buffer_view.clear();
                            self.com = Some(Box::new(t));
                            self.cur_addr = i;
                            self.connection_time = SystemTime::now();
                            let adr = self.addresses[i].clone();
                            self.auto_login = AutoLogin::new(adr.auto_login);
                            self.auto_login.disabled = self.is_alt_pressed;
                            self.buffer_view.buf.clear();
                            if let Some(mode) = &adr.screen_mode {
                                self.set_screen_mode(mode);
                            } else {
                                self.set_screen_mode(&ScreenMode::DOS(80, 25));
                            }
                            if let Some(font) = &adr.font_name {
                                self.set_font(font);
                            }
                            self.buffer_view.buffer_parser = self.addresses[i].get_terminal_parser();
                            self.print_log("Logged in".to_string());

                        },
                        Err(err) => {
                            eprintln!("{}", err);
                            self.print_log(format!("{}", err));
                            self.com = None;
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("{}", error);
                self.print_log(format!("Socket error: {:?}", error));
            }
        }
    }
}

