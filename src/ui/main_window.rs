use clipboard::{ClipboardContext, ClipboardProvider};
use iced::mouse::ScrollDelta;
use iced::widget::text_input;
use iced::widget::{column, text, Canvas, Row};
use iced::{executor, keyboard, mouse, subscription, Alignment, Event};
use iced::{Application, Command, Element, Length, Subscription, Theme};
use icy_engine::{BitFont, DEFAULT_FONT_NAME};
use rfd::FileDialog;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::address::{start_read_book, store_phone_book, Address, READ_ADDRESSES};
use crate::auto_file_transfer::AutoFileTransfer;
use crate::auto_login::AutoLogin;
use crate::com::{Com, TelnetCom};
use crate::protocol::{FileDescriptor, Protocol, TransferState};
use crate::VERSION;

use super::screen_modes::ScreenMode;
use super::{
    create_icon_button, BufferView, Message, ANSI_KEY_MAP, ATASCII_KEY_MAP, C64_KEY_MAP, CTRL_MOD,
    SHIFT_MOD, VT500_KEY_MAP, VIDEOTERM_KEY_MAP,
};

pub enum MainWindowMode {
    ShowTerminal,
    ShowPhonebook,
    SelectProtocol(bool),
    FileTransfer(bool),
    EditBBS(usize),
}

struct Options {
    connect_timeout: Duration,
}

impl Options {
    pub fn new() -> Self {
        Options {
            connect_timeout: Duration::from_secs(10),
        }
    }
}

pub struct MainWindow {
    pub buffer_view: BufferView,
    com: Option<Box<dyn Com>>,
    trigger: bool,
    pub mode: MainWindowMode,
    pub addresses: Vec<Address>,
    pub handled_char: bool,
    edit_bbs: Address,
    cur_addr: usize,
    options: Options,
    connection_time: SystemTime,
    font: Option<String>,
    screen_mode: Option<ScreenMode>,
    auto_login: AutoLogin,
    auto_file_transfer: AutoFileTransfer,
    // protocols
    current_protocol: Option<(Box<dyn Protocol>, TransferState)>,
    is_alt_pressed: bool,
}

impl MainWindow {
    pub fn update_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &mut self.com {
            None => Ok(()),
            Some(com) => {
                self.auto_login.disabled |= self.is_alt_pressed;
                if let Some(adr) = self.addresses.get(self.cur_addr) {
                    if let Err(err) = self.auto_login.run_autologin(com, adr) {
                        eprintln!("{}", err);
                    }
                }
                let mut do_update = false;
                let mut i = 0;
                // needed an upper limit for sixels - could really be much data in there
                while com.is_data_available()? && i < 2048 {
                    i = i + 1;
                    let ch = com.read_char_nonblocking()?;
                    if let Some(adr) = self.addresses.get(self.cur_addr) {
                        if let Err(err) = self.auto_login.try_login(com, adr, ch) {
                            eprintln!("{}", err);
                        }
                    }

                    self.buffer_view.print_char(Some(com.as_mut()), ch)?;
                    do_update = true;
                    if let Some((protocol_type, download)) =
                        self.auto_file_transfer.try_transfer(ch)
                    {
                        //                        if !download {
                        //                            self.mode = MainWindowMode::SelectProtocol(download);
                        //                        } else {
                        self.initiate_file_transfer(protocol_type, download);
                        //                        }
                        return Ok(());
                    }
                }
                if do_update {
                    self.buffer_view.redraw_view();
                }
                Ok(())
            }
        }
    }

    pub fn get_screen_mode(&self) -> ScreenMode {
        if let Some(mode) = self.screen_mode {
            return mode;
        }

        return ScreenMode::DOS(80, 25);
    }

    pub fn get_font_name(&self) -> String {
        if let Some(font) = &self.font {
            return font.clone();
        }

        return DEFAULT_FONT_NAME.to_string();
    }

    pub fn set_font(&mut self, font: &String) {
        if font != &self.get_font_name() {
            self.font = Some(font.clone());
            self.buffer_view.buf.font = BitFont::from_name(&self.get_font_name()).unwrap();
            self.buffer_view.redraw_view();
        }
    }

    pub fn set_screen_mode(&mut self, mode: &ScreenMode) {
        self.screen_mode = Some(*mode);
        self.get_screen_mode()
            .set_mode(&mut self.font, &mut self.buffer_view);
        self.buffer_view.buf.font = BitFont::from_name(&self.get_font_name()).unwrap();
        self.buffer_view.redraw_view();
    }

    pub fn output_char(&mut self, ch: char) {
        let translated_char = self.buffer_view.buffer_parser.from_unicode(ch);
        if let Some(com) = &mut self.com {
            let state = com.write(&[translated_char as u8]);
            if let Err(err) = state {
                eprintln!("{}", err);
                self.com = None;
            }
        } else {
            log_result(&self.buffer_view.print_char(None, translated_char as u8));
            self.buffer_view.redraw_view();
        }
    }

    pub fn println(&mut self, str: &str) {
        for c in str.chars() {
            log_result(&self.buffer_view.print_char(None, c as u8));
        }
        log_result(&self.buffer_view.print_char(None, b'\r'));
        log_result(&self.buffer_view.print_char(None, b'\n'));
    }

    fn initiate_file_transfer(
        &mut self,
        protocol_type: crate::protocol::ProtocolType,
        download: bool,
    ) {
        self.mode = MainWindowMode::ShowTerminal;
        match self.com.as_mut() {
            Some(com) => {
                if !download {
                    let files = FileDialog::new().pick_files();
                    if let Some(path) = files {
                        let fd = FileDescriptor::from_paths(&path);
                        if let Ok(files) = fd {
                            let mut protocol = protocol_type.create();
                            match protocol.initiate_send(com, files) {
                                Ok(state) => {
                                    self.mode = MainWindowMode::FileTransfer(download);
                                    self.current_protocol = Some((protocol, state));
                                }
                                Err(error) => {
                                    eprintln!("{}", error);
                                }
                            }
                        } else {
                            log_result(&fd);
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
                        }
                    }
                }
            }
            None => {
                eprintln!("Communication error.");
            }
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
            let d = SystemTime::now()
                .duration_since(self.connection_time)
                .unwrap();
            let sec = d.as_secs();
            let minutes = sec / 60;
            let hours = minutes / 60;
            let cur = &self.addresses[self.cur_addr];

            format!(
                "Connected {:02}:{:02}:{:02} to {}",
                hours,
                minutes % 60,
                sec % 60,
                if cur.system_name.len() > 0 {
                    &cur.system_name
                } else {
                    &cur.address
                }
            )
        } else {
            "Offline".to_string()
        };
        format!("iCY TERM {} - {}", VERSION, str)
    }

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let mut view = MainWindow {
            buffer_view: BufferView::new(),
            com: None,
            trigger: true,
            mode: MainWindowMode::ShowPhonebook,
            addresses: start_read_book(),
            edit_bbs: Address::new(),
            cur_addr: 0,
            connection_time: SystemTime::now(),
            options: Options::new(),
            auto_login: AutoLogin::new(String::new()),
            auto_file_transfer: AutoFileTransfer::new(),
            font: Some(DEFAULT_FONT_NAME.to_string()),
            screen_mode: None,
            current_protocol: None,
            handled_char: false,
            is_alt_pressed: false,
        };
        

        let args: Vec<String> = env::args().collect();
        if let Some(arg) = args.get(1) {
            view.addresses[0].address = arg.clone();
            let cmd = view.call_bbs(0);
            return (view, cmd);
        }
        (view, Command::none())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.trigger = !self.trigger;
        if unsafe { READ_ADDRESSES } {
            unsafe {
                READ_ADDRESSES = false;
            }
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
       
        // view.set_screen_mode(&ScreenMode::Viewdata);
        // unsafe { super::simulate::run_sim(self); }
        
        match &message {
            Message::OpenURL(url) => {
                if let Err(err) = open::that(url) {
                    eprintln!("{}", err);
                }
            }
            Message::Connected(t) => match t {
                Ok(_addr) => {
                    unsafe {
                        self.com = COM2.replace(None).unwrap();
                    }
                    self.buffer_view.clear();
                    self.connection_time = SystemTime::now();
                }
                Err(err) => {
                    eprintln!("{}", err);
                    self.println(err.to_string().as_str());
                    self.com = None;
                }
            },
            _ => {}
        };

        match self.mode {
            MainWindowMode::ShowTerminal => {
                match message {
                    Message::InitiateFileTransfer(download) => {
                        self.mode = MainWindowMode::SelectProtocol(download);
                    }
                    Message::SendLogin => {
                        if let Some(com) = &mut self.com {
                            let adr = self.addresses.get(self.cur_addr).unwrap();
                            if let Err(err) = com.write(
                                [
                                    adr.user_name.as_bytes(),
                                    b"\r",
                                    adr.password.as_bytes(),
                                    b"\r",
                                ]
                                .concat()
                                .as_slice(),
                            ) {
                                eprintln!("Error sending login: {}", err);
                            }
                            self.auto_login.logged_in = true;
                        }
                    }
                    Message::Hangup => {
                        self.com = None;
                        self.mode = MainWindowMode::ShowPhonebook;
                    }
                    Message::Tick => {
                        let state = self.update_state();
                        if let Err(err) = state {
                            eprintln!("{}", err);
                        }
                    }
                    Message::CharacterReceived(ch) => {
                        if self.handled_char {
                            self.handled_char = false;
                        } else {
                            self.output_char(ch);
                        }
                    }
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
                        let map = match self.buffer_view.buffer_input_mode {
                            super::BufferInputMode::CP437 => ANSI_KEY_MAP,
                            super::BufferInputMode::PETSCII => C64_KEY_MAP,
                            super::BufferInputMode::ATASCII => ATASCII_KEY_MAP,
                            super::BufferInputMode::VT500 => VT500_KEY_MAP,
                            super::BufferInputMode::VIEWDATA => VIDEOTERM_KEY_MAP,

                            
                        };

                        if let Some(com) = &mut self.com {
                            for (k, m) in map {
                                if *k == code {
                                    self.handled_char = true;
                                    let state = com.write(m);
                                    if let Err(err) = state {
                                        eprintln!("{}", err);
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
                                        if let Err(err) = state {
                                            eprintln!("{}", err);
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
                            self.buffer_view.redraw_view();
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
            }
            MainWindowMode::ShowPhonebook => {
                text_input::focus::<Message>(super::INPUT_ID.clone());
                match message {
                    Message::EditBBS(i) => {
                        self.edit_bbs = if i == 0 {
                            Address::new()
                        } else {
                            self.addresses[i].clone()
                        };
                        self.mode = MainWindowMode::EditBBS(i)
                    }

                    Message::CallBBS(i) => {
                        return self.call_bbs(i);
                    }

                    Message::QuickConnectChanged(addr) => self.addresses[0].address = addr,
                    _ => {}
                }
            }
            MainWindowMode::SelectProtocol(_) => match message {
                Message::Back => self.mode = MainWindowMode::ShowTerminal,
                Message::SelectProtocol(protocol_type, download) => {
                    self.initiate_file_transfer(protocol_type, download);
                }
                _ => {}
            },
            MainWindowMode::FileTransfer(_) => {
                match message {
                    Message::Tick => {
                        if let Some(com) = self.com.as_mut() {
                            if let Some((protocol, state)) = &mut self.current_protocol {
                                match protocol.update(com, state) {
                                    Err(err) => {
                                        eprintln!("Err {}", err);
                                    }
                                    _ => {}
                                }
                                // self.print_result(&r);
                                if state.is_finished {
                                    for f in protocol.get_received_files() {
                                        f.save_file_in_downloads(
                                            state.recieve_state.as_mut().unwrap(),
                                        )
                                        .expect("error saving file.");
                                    }
                                    self.mode = MainWindowMode::ShowTerminal;
                                    self.auto_file_transfer.reset();
                                }
                            }
                        }
                    }
                    Message::Back => {
                        self.current_protocol = None;
                        self.mode = MainWindowMode::ShowTerminal;
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
                    }

                    Message::EditBbsSystemNameChanged(str) => self.edit_bbs.system_name = str,
                    Message::EditBbsAddressChanged(str) => self.edit_bbs.address = str,
                    Message::EditBbsUserNameChanged(str) => self.edit_bbs.user_name = str,
                    Message::EditBbsPasswordChanged(str) => self.edit_bbs.password = str,
                    Message::EditBbsCommentChanged(str) => self.edit_bbs.comment = str,
                    Message::EditBbsTerminalTypeSelected(terminal) => {
                        self.edit_bbs.terminal_type = terminal
                    }
                    Message::EditBbsScreenModeSelected(screen_mode) => {
                        self.edit_bbs.screen_mode = Some(screen_mode)
                    }
                    Message::EditBbsAutoLoginChanged(str) => self.edit_bbs.auto_login = str,
                    Message::EditBbsConnectionType(connection_type) => {
                        self.edit_bbs.connection_type = connection_type
                    }
                    Message::EditBbsSaveChanges(i) => {
                        if i == 0 {
                            self.addresses.push(self.edit_bbs.clone());
                        } else {
                            self.addresses[i] = self.edit_bbs.clone();
                        }
                        log_result(&store_phone_book(&self.addresses));
                        self.mode = MainWindowMode::ShowPhonebook;
                    }
                    Message::EditBbsDeleteEntry(i) => {
                        if i > 0 {
                            self.addresses.remove(i);
                        }
                        log_result(&store_phone_book(&self.addresses));
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
            (
                Event::Keyboard(keyboard::Event::CharacterReceived(ch)),
                iced::event::Status::Ignored,
            ) => Some(Message::CharacterReceived(ch)),
            (
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                    ..
                }),
                iced::event::Status::Ignored,
            ) => Some(Message::KeyPressed(key_code, modifiers)),
            (
                Event::Keyboard(keyboard::Event::KeyReleased {
                    key_code,
                    modifiers,
                    ..
                }),
                iced::event::Status::Ignored,
            ) => Some(Message::KeyReleased(key_code, modifiers)),
            (
                Event::Mouse(mouse::Event::WheelScrolled { delta, .. }),
                iced::event::Status::Ignored,
            ) => Some(Message::WheelScrolled(delta)),
            /*(Event::Window(ev), iced::event::Status::Ignored) => {
                println!("{:?}",ev );
                None
            },*/
            _ => None,
        });

        let t = iced::time::every(std::time::Duration::from_millis(10)).map(|_| Message::Tick);

        Subscription::<Message>::batch([s, t])
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        match self.mode {
            MainWindowMode::ShowTerminal => self.view_terminal_window(),
            MainWindowMode::ShowPhonebook => super::view_phonebook(self),
            MainWindowMode::SelectProtocol(download) => super::view_protocol_selector(download),
            MainWindowMode::EditBBS(i) => super::view_edit_bbs(self, &self.edit_bbs, i),
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

static UPLOAD_SVG: &[u8] = include_bytes!("../../resources/upload.svg");
static DOWNLOAD_SVG: &[u8] = include_bytes!("../../resources/download.svg");
static KEY_SVG: &[u8] = include_bytes!("../../resources/key.svg");
static LOGOUT_SVG: &[u8] = include_bytes!("../../resources/logout.svg");

impl MainWindow {
    pub fn view_terminal_window(&self) -> Element<'_, Message> {
        let c = Canvas::new(&self.buffer_view)
            .width(Length::Fill)
            .height(Length::Fill);

        let mut title_row = Row::new();
        if self.com.is_some() {
            title_row = title_row.push(
                create_icon_button(UPLOAD_SVG).on_press(Message::InitiateFileTransfer(false)),
            );
            title_row = title_row.push(
                create_icon_button(DOWNLOAD_SVG).on_press(Message::InitiateFileTransfer(true)),
            );
            if !self.auto_login.logged_in {
                title_row =
                    title_row.push(create_icon_button(KEY_SVG).on_press(Message::SendLogin));
            }
        }
        title_row = title_row.push(create_icon_button(LOGOUT_SVG).on_press(Message::Hangup));
        column(vec![
            title_row
                .align_items(Alignment::Center)
                .spacing(8)
                .padding(4)
                .into(),
            c.into(),
        ])
        .into()
    }
}

impl MainWindow {
    fn call_bbs(&mut self, i: usize) -> Command<Message> {
        self.mode = MainWindowMode::ShowTerminal;
        let mut adr = self.addresses[i].address.clone();
        if !adr.contains(":") {
            adr.push_str(":23");
        }

        let call_adr = self.addresses[i].clone();
        self.auto_login = AutoLogin::new(call_adr.auto_login);
        self.auto_login.disabled = self.is_alt_pressed;
        self.buffer_view.buf.clear();
        self.cur_addr = i;
        if let Some(mode) = &call_adr.screen_mode {
            self.set_screen_mode(mode);
        } else {
            self.set_screen_mode(&ScreenMode::DOS(80, 25));
        }
        if let Some(font) = &call_adr.font_name {
            self.set_font(font);
        }
        self.buffer_view.buffer_parser = self.addresses[i].get_terminal_parser();
        self.println(&format!("Connect to {}...", &call_adr.address));
        let a = call_adr.address.clone();
        unsafe {
            COM2 = Some(Some(Box::new(TelnetCom::new())));
        }
        Command::perform(foo(a), Message::Connected)
    }
}

pub fn log_result<'a, T>(result: &Result<T, Box<dyn std::error::Error + 'a>>) {
    if let Err(error) = result {
        eprintln!("{}", error);
    }
}

static mut COM2: Option<Option<Box<dyn Com + 'static>>> = None;

async fn foo(a: String) -> Result<bool, String> {
    unsafe {
        let mut c = COM2.replace(None);
        println!("Connect…");
        c.as_mut().unwrap().as_mut().unwrap().connect(a).await?;
        println!("success!!!");
        COM2 = c;
    }

    Ok(true)
}
