#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(unsafe_code)]

use std::{sync::Arc, env};
use egui::mutex::Mutex;
use icy_engine::{DEFAULT_FONT_NAME, BufferParser, AvatarParser};
use rfd::FileDialog;
use tokio::{runtime::{Runtime, self}, task::JoinHandle};
use std::time::{Duration, SystemTime};

use eframe::{egui::{self, Key}};

use crate::{address::{Address, start_read_book}, com::{TelnetCom, RawCom, SSHCom}, protocol::FileDescriptor};
use crate::auto_file_transfer::AutoFileTransfer;
use crate::auto_login::AutoLogin;
use crate::com::{Com};
use crate::protocol::{Protocol, TransferState};

use super::{BufferView, screen_modes::ScreenMode};

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
    pub rt: Runtime,
    pub com: Option<Box<dyn Com>>,
    pub buffer_view: Arc<Mutex<BufferView>>,
    pub buffer_parser: Box<dyn BufferParser>,
    
    trigger: bool,
    pub mode: MainWindowMode,
    pub addresses: Vec<Address>,
    pub handled_char: bool,
    cur_addr: usize,
    options: Options,
    connection_time: SystemTime,
    font: Option<String>,
    pub screen_mode: ScreenMode,
    auto_login: AutoLogin,
    auto_file_transfer: AutoFileTransfer,
    // protocols
    current_protocol: Option<(Box<dyn Protocol>, TransferState)>,
    is_alt_pressed: bool,
    call_bbs_handle: Option<JoinHandle<Result<bool, String>>>
}

impl MainWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        
        let view  = BufferView::new(gl);

        let mut view = MainWindow {
            rt: runtime::Builder::new_multi_thread().enable_all().build().unwrap(),
            buffer_view: Arc::new(Mutex::new(view)),
            //address_list: HoverList::new(),
            com: None,
            trigger: true,
            mode: MainWindowMode::ShowTerminal,
            addresses: start_read_book(),
            cur_addr: 0,
            connection_time: SystemTime::now(),
            options: Options::new(),
            auto_login: AutoLogin::new(String::new()),
            auto_file_transfer: AutoFileTransfer::new(),
            font: Some(DEFAULT_FONT_NAME.to_string()),
            screen_mode: ScreenMode::DOS(80, 25),
            current_protocol: None,
            handled_char: false,
            is_alt_pressed: false,
            buffer_parser: Box::new(AvatarParser::new(true)),
            call_bbs_handle: None
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
                if let Some(com) = &mut self.com {
                    com.write(result.as_bytes())?;
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
        }
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
        unsafe {
            let com:Box<dyn Com> = match call_adr.connection_type {
                crate::address::ConnectionType::Telnet => Box::new(TelnetCom::new()),
                crate::address::ConnectionType::Raw => Box::new(RawCom::new()),
                crate::address::ConnectionType::SSH => Box::new(SSHCom::new()),
            };
            COM2 = Some(Some(com));
        }

        let time_out  = self.options.connect_timeout;
        self.call_bbs_handle = Some(self.rt.spawn(async move {
            foo(call_adr, time_out).await
        }));
    }

    pub fn update_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {

        unsafe { super::simulate::run_sim(self); }

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
                    let result = self.buffer_view.lock().print_char(&mut self.buffer_parser, unsafe { char::from_u32_unchecked(ch as u32) })?;
                    match result {
                        icy_engine::CallbackAction::None => {},
                        icy_engine::CallbackAction::SendString(result) => {
                            com.write(result.as_bytes())?;
                        },
                        icy_engine::CallbackAction::PlayMusic(music) => { /* play_music(music)*/ }
                    }
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
                    self.buffer_view.lock().redraw_view();
                    println!("do update!")
                }
                Ok(())
            }
        }
    }

    pub fn hangup(&mut self) {
        self.com = None;
        self.mode = MainWindowMode::ShowPhonebook;
    }

    pub fn send_login(&mut self) {
        if let Some(com) = &mut self.com {
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
    
            if let Err(err) = com.write(&data) {
                eprintln!("Error sending login: {}", err);
            }
            self.auto_login.logged_in = true;
        }
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(handle) = &self.call_bbs_handle {
            if handle.is_finished() {
                unsafe {
                    self.com = COM2.replace(None).unwrap();
                }
                self.buffer_view.lock().buf.clear();
                self.connection_time = SystemTime::now();

                self.call_bbs_handle = None;
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

static mut COM2: Option<Option<Box<dyn Com + 'static>>> = None;

async fn foo(addr: Address, timeout: Duration) -> Result<bool, String> {
    unsafe {
        let mut c = COM2.replace(None);
        c.as_mut().unwrap().as_mut().unwrap().connect(&addr, timeout).await?;
        COM2 = c;
    }

    Ok(true)
}