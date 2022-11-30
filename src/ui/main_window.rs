#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(unsafe_code)]

use std::{sync::{Arc}, env};
use egui::mutex::Mutex;
use icy_engine::{DEFAULT_FONT_NAME, BufferParser, AvatarParser};
use poll_promise::Promise;
use std::time::{Duration, SystemTime};

use eframe::{egui::{self, Key}};

use crate::{address::{Address, start_read_book}, com::{TelnetCom, RawCom, SSHCom, SendData}, TerminalResult};
use crate::auto_file_transfer::AutoFileTransfer;
use crate::auto_login::AutoLogin;
use crate::com::{Com};
use crate::protocol::{Protocol, TransferState};

use super::{BufferView, screen_modes::ScreenMode};
use tokio::sync::mpsc;
use crate::{com::{Connection}};

#[derive(PartialEq, Eq)]
pub enum MainWindowMode {
    ShowTerminal,
    ShowPhonebook,
    SelectProtocol(bool),
    FileTransfer(bool),
    AskDeleteEntry
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
    pub buffer_view: Arc<Mutex<BufferView>>,
    pub buffer_parser: Box<dyn BufferParser>,

    pub connection_opt: Option<Connection>,
    
    trigger: bool,
    pub mode: MainWindowMode,
    pub addresses: Vec<Address>,
    pub handled_char: bool,
    cur_addr: usize,
    options: Options,
    font: Option<String>,
    pub screen_mode: ScreenMode,
    auto_login: AutoLogin,
    auto_file_transfer: AutoFileTransfer,
    // protocols
    current_protocol: Option<(Box<dyn Protocol>, TransferState)>,
    is_alt_pressed: bool,

    open_connection_promise: Option<Promise<Box<dyn Com>>>,
}

impl MainWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        
        let view  = BufferView::new(gl);

        let mut view = MainWindow {
            buffer_view: Arc::new(Mutex::new(view)),
            //address_list: HoverList::new(),
            trigger: true,
            mode: MainWindowMode::ShowPhonebook,
            addresses: start_read_book(),
            cur_addr: 0,
            connection_opt: None,
            options: Options::new(),
            auto_login: AutoLogin::new(String::new()),
            auto_file_transfer: AutoFileTransfer::new(),
            font: Some(DEFAULT_FONT_NAME.to_string()),
            screen_mode: ScreenMode::DOS(80, 25),
            current_protocol: None,
            handled_char: false,
            is_alt_pressed: false,
            buffer_parser: Box::new(AvatarParser::new(true)),
            open_connection_promise: None
        };
        let args: Vec<String> = env::args().collect();
        if let Some(arg) = args.get(1) {
            view.addresses[0].address = arg.clone();
            view.call_bbs(0);
        }
        //view.address_list.selected_item = 1;
        // view.set_screen_mode(&ScreenMode::Viewdata);
        //view.update_address_list();
        unsafe {
            super::simulate::run_sim(&mut view); 
        }

        view
    }


    pub fn println(&mut self, str: &str) -> Result<(), Box<dyn std::error::Error>> {
        for c in str.chars() {
            self.buffer_view.lock().print_char(&mut self.buffer_parser, unsafe { char::from_u32_unchecked(c as u32) })?;
        }
        Ok(())
    }

    pub fn output_char(&mut self, ch: char) {
        let translated_char = self.buffer_parser.from_unicode(ch);
        if let Some(con) = &mut self.connection_opt {
            con.send(vec![translated_char as u8]);
        } else {
            self.print_char(translated_char as u8);
        }
    }

    pub fn print_char(
        &mut self,
        c: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        /* 
        match c  {
            b'\\' => print!("\\\\"),
            b'\n' => print!("\\n"),
            b'\r' => print!("\\r"),
            b'\"' => print!("\\\""),
            _ => {
                if c < b' ' || c == b'\x7F' {
                    print!("\\x{:02X}", c as u8);
                } else if c > b'\x7F' {
                    print!("\\u{{{:02X}}}", c as u8);
                } else {
                    print!("{}", char::from_u32(c as u32).unwrap());
                }
            }
        }*/
        
        let result = self.buffer_view.lock().print_char(&mut self.buffer_parser, unsafe { char::from_u32_unchecked(c as u32) })?;

        match result {
            icy_engine::CallbackAction::None => {},
            icy_engine::CallbackAction::SendString(result) => {
                if let Some(con) = &mut self.connection_opt {
                    con.send(result.as_bytes().to_vec());
                }
            },
            icy_engine::CallbackAction::PlayMusic(music) => { /* play_music(music)*/ }
        }
        //if !self.update_sixels() {
            self.buffer_view.lock().redraw_view();
        //}
        Ok(())
    }

    pub(crate) fn initiate_file_transfer(&mut self, protocol_type: crate::protocol::ProtocolType, download: bool) {
        self.mode = MainWindowMode::ShowTerminal;
       /*  match self.com.as_mut() {
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
                         //   log_result(&fd);
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
        }*/
    }
        
    pub fn set_screen_mode(&mut self, mode: ScreenMode) {
        self.screen_mode = mode;
        mode.set_mode(self);
    }

    pub fn call_bbs(&mut self, i: usize) {
        self.mode = MainWindowMode::ShowTerminal;
        let mut adr = self.addresses[i].address.clone();
        if !adr.contains(":") {
            adr.push_str(":23");
        }

        let call_adr = self.addresses[i].clone();
        self.auto_login = AutoLogin::new(call_adr.auto_login.clone());
        self.auto_login.disabled = self.is_alt_pressed;
        self.buffer_view.lock().buf.clear();
        self.cur_addr = i;
        if let Some(mode) = &call_adr.screen_mode {
            self.set_screen_mode(*mode);
        } else {
            self.set_screen_mode(ScreenMode::DOS(80, 25));
        }
        self.buffer_parser = self.addresses[i].get_terminal_parser();
        self.println(&format!("Connect to {}...", &call_adr.address));
      
        let timeout  = self.options.connect_timeout;
        let ct  = call_adr.connection_type;
        self.open_connection_promise = Some(Promise::spawn_async(async move {
            let mut com: Box<dyn Com> = match ct {
                crate::address::ConnectionType::Telnet => Box::new(TelnetCom::new()),
                crate::address::ConnectionType::Raw => Box::new(RawCom::new()),
                crate::address::ConnectionType::SSH => Box::new(SSHCom::new()),
            };
            com.connect(&call_adr, timeout).await;
            com
        }));
    }

    pub fn update_state(&mut self) -> TerminalResult<()> {
//        unsafe { super::simulate::run_sim(self); }
        let Some(con) = &mut self.connection_opt else { return Ok(()) };

        if con.is_data_available()? {
            if let Ok(vec) = con.read_buffer() {
                for ch in vec { 
                    if let Some(adr) = self.addresses.get(self.cur_addr) {
                        if let Err(err) = self.auto_login.try_login( con, adr, ch) {
                            eprintln!("{}", err);
                        }
                    }
                    let result = self.buffer_view.lock().print_char(&mut self.buffer_parser, unsafe { char::from_u32_unchecked(ch as u32) })?;
                    match result {
                        icy_engine::CallbackAction::None => {},
                        icy_engine::CallbackAction::SendString(result) => {
                            con.send(result.as_bytes().to_vec());
                        },
                        icy_engine::CallbackAction::PlayMusic(music) => { /* play_music(music)*/ }
                    }
                    if let Some((protocol_type, download)) =
                        self.auto_file_transfer.try_transfer(ch)
                    {
                        self.initiate_file_transfer(protocol_type, download);
                        return Ok(());
                    }
                }
            }
        }
        if con.is_disconnected() {
            self.connection_opt = None;
        }
        self.auto_login.disabled |= self.is_alt_pressed;
        if let Some(adr) = self.addresses.get(self.cur_addr) {
            if let Some(con) = &mut self.connection_opt {
                if let Err(err) = self.auto_login.run_autologin(con, adr) {
                    eprintln!("{}", err);
                }
            }
        }
        
        Ok(())
    }

    pub fn hangup(&mut self) {
        self.open_connection_promise = None;
        if let Some(con) = &mut self.connection_opt {
            con.disconnect();
        }
        self.connection_opt = None;
        self.mode = MainWindowMode::ShowPhonebook;
    }

    pub fn send_login(&mut self) {
        let adr = self.addresses.get(self.cur_addr).unwrap();
        let mut cr = [self.buffer_parser.from_unicode('\r') as u8].to_vec();
        for (k, v) in self.screen_mode.get_input_mode().cur_map() {
            if *k == Key::Enter as u32 {
                cr = v.to_vec();
                break;
            }
        }
        let mut data = Vec::new();
        data.extend_from_slice(adr.user_name.as_bytes());
        data.extend(&cr);
        data.extend_from_slice(adr.password.as_bytes());
        data.extend(cr);
        if let Some(con) = &mut self.connection_opt {
            con.send(data);
        }
        self.auto_login.logged_in = true;
    }

    fn update_title(&self, frame: &mut eframe::Frame) {
        match self.mode {
            MainWindowMode::ShowPhonebook => {
                frame.set_window_title(&crate::DEFAULT_TITLE);
            }
            _ => {
                let str = if let Some(con) = &self.connection_opt {
                    let d = SystemTime::now()
                        .duration_since(con.get_connection_time())
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
                frame.set_window_title(format!("iCY TERM {} - {}", crate::VERSION, str).as_str());
            }
        }
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.update_title(frame);

        if self.open_connection_promise.is_some() {
            if self.open_connection_promise.as_ref().unwrap().ready().is_some() {
                if let Ok(handle) = self.open_connection_promise.take().unwrap().try_take() {
                    self.open_connection_promise  = None;
                    let ctx = ctx.clone();
                    
                    let (tx, rx) = mpsc::channel::<SendData>(32);
                    let (tx2, mut rx2) = mpsc::channel::<SendData>(32);
                    self.connection_opt = Some(Connection::new(rx, tx2.clone()));

                    let mut handle = handle;
                    tokio::spawn(async move {
                        let mut done = false;
                        while !done {
                            let a = tokio::select! {
                                Ok(v) = handle.read_data() => {
                                    if let Err(err) = tx.send(SendData::Data(v)).await {
                                        eprintln!("error while sending: {}", err);
                                        done = true;
                                } else {
                                        ctx.request_repaint();
                                    }
                                }
                                result = rx2.recv() => {
                                    let msg = result.unwrap();
                                    match msg {
                                        SendData::Char(c) => { 
                                            if let Err(err) = handle.write(&[c as u8]).await {
                                                eprintln!("{}", err);
                                                done = true;
                                            }
                                        },
                                        SendData::Data(buf) => { 
                                            if let Err(err) = handle.write(&buf).await {
                                                eprintln!("{}", err);
                                                done = true;
                                            }
                                        },
                                        SendData::Disconnect => {
                                            done = true;
                                        }
                                    }
                                }
                            };
                        }
                        let a = tx.send(SendData::Disconnect).await;
                    });
                }
            }
        }

        self.update_state();

        match self.mode {
            MainWindowMode::ShowTerminal => self.update_terminal_window(ctx, frame),
            MainWindowMode::ShowPhonebook => {
                super::view_phonebook(self, ctx, frame); 
            },
            MainWindowMode::SelectProtocol(download) => {
                self.update_terminal_window(ctx, frame);
                super::view_protocol_selector(self, ctx, frame, download); 
            },
            MainWindowMode::FileTransfer(download) => {
                self.update_terminal_window(ctx, frame);
                todo!()
            },
            MainWindowMode::AskDeleteEntry => todo!(),
        }
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.buffer_view.lock().destroy(gl);
        }
    }
}
