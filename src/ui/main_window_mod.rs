#![allow(unsafe_code, clippy::wildcard_imports)]

use chrono::Utc;
use eframe::epaint::FontId;
use i18n_embed_fl::fl;
use icy_engine::{ansi, BufferParser};
use poll_promise::Promise;
use rfd::FileDialog;
use std::time::{Duration, SystemTime};
use std::{
    env,
    sync::{Arc, Mutex},
};

use eframe::egui::{self, Key};

use crate::auto_file_transfer::AutoFileTransfer;
use crate::auto_login::AutoLogin;
use crate::com::{Com, TermComResult};
use crate::protocol::TransferState;
use crate::rng::Rng;
use crate::{
    address_mod::{start_read_book, store_phone_book, Address},
    com::{ComRawImpl, ComTelnetImpl, SendData},
    protocol::FileDescriptor,
    TerminalResult,
};

use super::{screen_modes::ScreenMode, ViewState};
use super::{Options, PhonebookFilter};
use crate::com::Connection;
use tokio::sync::mpsc;

#[derive(PartialEq, Eq)]
pub enum MainWindowMode {
    ShowTerminal,
    ShowPhonebook,
    ShowSettings(bool),
    SelectProtocol(bool),
    FileTransfer(bool),
    //   AskDeleteEntry
}

pub struct MainWindow {
    pub buffer_view: Arc<eframe::epaint::mutex::Mutex<ViewState>>,
    pub buffer_parser: Box<dyn BufferParser>,

    pub connection_opt: Option<Connection>,

    pub mode: MainWindowMode,
    pub addresses: Vec<Address>,
    pub handled_char: bool,
    cur_addr: usize,
    pub selected_bbs: Option<usize>,
    pub phonebook_filter: PhonebookFilter,
    pub phonebook_filter_string: String,

    pub options: Options,
    pub screen_mode: ScreenMode,
    pub auto_login: AutoLogin,

    pub rng: Rng,
    auto_file_transfer: AutoFileTransfer,
    // protocols
    current_transfer: Option<Arc<Mutex<TransferState>>>,
    is_alt_pressed: bool,

    open_connection_promise: Option<Promise<TermComResult<Box<dyn Com>>>>,
}

impl MainWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        let options = Options::load_options();
        let view = ViewState::new(gl, &options);
        let mut view = MainWindow {
            buffer_view: Arc::new(eframe::epaint::mutex::Mutex::new(view)),
            //address_list: HoverList::new(),
            mode: MainWindowMode::ShowPhonebook,
            addresses: start_read_book(),
            cur_addr: 0,
            selected_bbs: None,
            connection_opt: None,
            options,
            auto_login: AutoLogin::new(""),
            auto_file_transfer: AutoFileTransfer::new(),
            screen_mode: ScreenMode::Vga(80, 25),
            current_transfer: None,
            handled_char: false,
            is_alt_pressed: false,
            phonebook_filter: PhonebookFilter::All,
            buffer_parser: Box::<ansi::Parser>::default(),
            open_connection_promise: None,
            phonebook_filter_string: String::new(),
            rng: Rng::new(),
        };
        let args: Vec<String> = env::args().collect();
        if let Some(arg) = args.get(1) {
            view.addresses[0].address = arg.clone();
            view.call_bbs(0);
        }

        //view.address_list.selected_item = 1;
        // view.set_screen_mode(&ScreenMode::Viewdata);
        //view.update_address_list();
        /*
        unsafe {
            view.mode = MainWindowMode::ShowTerminal;
            super::simulate::run_sim(&mut view);
        }*/
        view
    }

    pub fn println(&mut self, str: &str) -> TerminalResult<()> {
        for ch in str.chars() {
            if ch as u32 > 255 {
                continue;
            }
            self.buffer_view
                .lock()
                .print_char(&mut self.buffer_parser, ch)?;
        }
        Ok(())
    }

    pub fn handle_result<T>(&mut self, res: TerminalResult<T>, terminate_connection: bool) {
        if let Err(err) = res {
            //            self.hangup();
            //            self.buffer_view.lock().buf.clear();
            self.println(&format!("\n\r{err}")).unwrap();
            eprintln!("{err}");
            if let Some(con) = &mut self.connection_opt {
                if con.is_disconnected() {
                    self.connection_opt = None;
                    self.open_connection_promise = None;
                    self.output_string(&format!("\n{err}"));
                }
            }

            if terminate_connection {
                self.open_connection_promise = None;
                if let Some(con) = &mut self.connection_opt {
                    con.disconnect().unwrap_or_default();
                }
                self.connection_opt = None;
            }
        }
    }

    pub fn output_char(&mut self, ch: char) {
        let translated_char = self.buffer_parser.convert_from_unicode(ch);
        self.buffer_view.lock().selection_opt = None;
        if let Some(con) = &mut self.connection_opt {
            let r = con.send(vec![translated_char as u8]);
            self.handle_result(r, false);
        } else if let Err(err) = self.print_char(translated_char as u8) {
            eprintln!("{err}");
        }
    }

    pub fn output_string(&mut self, str: &str) {
        self.buffer_view.lock().selection_opt = None;
        if let Some(con) = &mut self.connection_opt {
            let mut v = Vec::new();
            for ch in str.chars() {
                let translated_char = self.buffer_parser.convert_from_unicode(ch);
                v.push(translated_char as u8);
            }
            let r = con.send(v);
            self.handle_result(r, false);
        } else {
            for ch in str.chars() {
                let translated_char = self.buffer_parser.convert_from_unicode(ch);
                if let Err(err) = self.print_char(translated_char as u8) {
                    eprintln!("{err}");
                }
            }
        }
    }

    pub fn print_char(&mut self, c: u8) -> Result<(), Box<dyn std::error::Error>> {
        let result = self
            .buffer_view
            .lock()
            .print_char(&mut self.buffer_parser, unsafe {
                char::from_u32_unchecked(c as u32)
            })?;
        match result {
            icy_engine::CallbackAction::None => {}
            icy_engine::CallbackAction::SendString(result) => {
                if let Some(con) = &mut self.connection_opt {
                    let r = con.send(result.as_bytes().to_vec());
                    self.handle_result(r, false);
                }
            }
            icy_engine::CallbackAction::PlayMusic(_music) => {
                //play_music(&music),
            }
            icy_engine::CallbackAction::Beep => {
                //crate::sound::beep()
                println!("beep.");
            }
        }
        //if !self.update_sixels() {
        self.buffer_view.lock().redraw_view();
        //}
        Ok(())
    }

    fn start_transfer_thread(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        files_opt: Option<Vec<FileDescriptor>>,
    ) {
        self.mode = MainWindowMode::FileTransfer(download);
        let state = Arc::new(Mutex::new(TransferState::new()));
        self.current_transfer = Some(state.clone());
        let res = self.connection_opt.as_mut().unwrap().start_file_transfer(
            protocol_type,
            download,
            state,
            files_opt,
        );
        self.handle_result(res, true);
    }

    /*

                                    let mut protocol = protocol_type.create();
                                match protocol.initiate_send(com, files, &self.current_transfer.unwrap()) {
                                    Ok(state) => {
                                        self.mode = MainWindowMode::FileTransfer(download);
    //                                    let a =(protocol, )));

    self.current_transfer = Some(Arc::new(Mutex::new(state)));
    }
                                    Err(error) => {
                                        eprintln!("{}", error);
                                    }
                                }

        */
    pub(crate) fn initiate_file_transfer(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
    ) {
        self.mode = MainWindowMode::ShowTerminal;
        match self.connection_opt.as_mut() {
            Some(_) => {
                if download {
                    self.start_transfer_thread(protocol_type, download, None);
                } else {
                    let files = FileDialog::new().pick_files();
                    if let Some(path) = files {
                        let fd = FileDescriptor::from_paths(&path);
                        if let Ok(files) = fd {
                            self.start_transfer_thread(protocol_type, download, Some(files));
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

    pub fn show_terminal(&mut self) {
        self.mode = MainWindowMode::ShowTerminal;
    }

    pub fn show_phonebook(&mut self) {
        self.mode = MainWindowMode::ShowPhonebook;
    }

    pub fn get_address_mut(&mut self, uuid: Option<usize>) -> &mut Address {
        if uuid.is_none() {
            return &mut self.addresses[0];
        }

        let uuid = uuid.unwrap();
        for (i, adr) in self.addresses.iter().enumerate() {
            if adr.id == uuid {
                return &mut self.addresses[i];
            }
        }

        &mut self.addresses[0]
    }

    pub fn call_bbs_uuid(&mut self, uuid: Option<usize>) {
        if uuid.is_none() {
            self.call_bbs(0);
            return;
        }

        let uuid = uuid.unwrap();
        for (i, adr) in self.addresses.iter().enumerate() {
            if adr.id == uuid {
                self.call_bbs(i);
                return;
            }
        }
    }

    pub fn call_bbs(&mut self, i: usize) {
        self.mode = MainWindowMode::ShowTerminal;
        let mut adr = self.addresses[i].address.clone();
        if !adr.contains(':') {
            adr.push_str(":23");
        }
        self.addresses[i].number_of_calls += 1;
        self.addresses[i].last_call = Some(Utc::now());
        store_phone_book(&self.addresses).unwrap_or_default();

        let call_adr = self.addresses[i].clone();
        self.auto_login = AutoLogin::new(&call_adr.auto_login);
        self.auto_login.disabled = self.is_alt_pressed;
        self.buffer_view.lock().buf.clear();
        self.cur_addr = i;
        self.set_screen_mode(call_adr.screen_mode);
        self.buffer_parser = self.addresses[i].get_terminal_parser();

        self.buffer_view.lock().redraw_font();
        self.buffer_view.lock().redraw_palette();
        self.buffer_view.lock().redraw_view();
        self.buffer_view.lock().clear();

        self.println(&fl!(
            crate::LANGUAGE_LOADER,
            "connect-to",
            address = call_adr.address.clone()
        ))
        .unwrap_or_default();

        let timeout = self.options.connect_timeout;
        let ct = call_adr.protocol;
        let window_size = self.screen_mode.get_window_size();

        self.open_connection_promise = Some(Promise::spawn_async(async move {
            let mut com: Box<dyn Com> = match ct {
                crate::address_mod::Protocol::Ssh | crate::address_mod::Protocol::Telnet => {
                    Box::new(ComTelnetImpl::new(window_size))
                }
                crate::address_mod::Protocol::Raw => Box::new(ComRawImpl::new()),
                // crate::address_mod::Protocol::Ssh => Box::new(crate::com::SSHCom::new()),
            };
            com.set_terminal_type(call_adr.terminal_type);
            if let Err(err) = com.connect(&call_adr, timeout).await {
                Err(err)
            } else {
                Ok(com)
            }
        }));
    }

    pub fn select_bbs(&mut self, uuid: Option<usize>) {
        self.selected_bbs = uuid;
    }

    pub fn delete_selected_address(&mut self) {
        if let Some(uuid) = self.selected_bbs {
            for (i, adr) in self.addresses.iter().enumerate() {
                if adr.id == uuid {
                    self.addresses.remove(i);
                    break;
                }
            }
        }
        let res = store_phone_book(&self.addresses);
        self.handle_result(res, true);
    }

    pub fn update_state(&mut self) -> TerminalResult<()> {
        //        unsafe { super::simulate::run_sim(self); }
        let Some(con) = &mut self.connection_opt else {
            return Ok(());
        };
        let mut send_data = Vec::new();

        if con.is_data_available()? {
            for ch in con.read_buffer() {
                if let Some(adr) = self.addresses.get(self.cur_addr) {
                    if let Err(err) = self.auto_login.try_login(con, adr, ch) {
                        eprintln!("{err}");
                    }
                }

                match ch {
                    b'\\' => print!("\\\\"),
                    b'\n' => println!("\\n"),
                    b'\r' => print!("\\r"),
                    b'\"' => print!("\\\""),
                    _ => {
                        if ch < b' ' || ch == b'\x7F' {
                            print!("\\x{ch:02X}");
                        } else if ch > b'\x7F' {
                            print!("\\u{{{ch:02X}}}");
                        } else {
                            print!("{}", char::from_u32(ch as u32).unwrap());
                        }
                    }
                }

                let result = self
                    .buffer_view
                    .lock()
                    .print_char(&mut self.buffer_parser, unsafe {
                        char::from_u32_unchecked(ch as u32)
                    });

                match result {
                    Ok(icy_engine::CallbackAction::None) => {}
                    Ok(icy_engine::CallbackAction::SendString(result)) => {
                        send_data.extend_from_slice(result.as_bytes());
                    }
                    Ok(icy_engine::CallbackAction::PlayMusic(_music)) => {
                        // play_music(&music)
                    }
                    Ok(icy_engine::CallbackAction::Beep) => {
                        // crate::sound::beep()
                        println!("beep.");
                    }
                    Err(err) => {
                        eprintln!("{err}");
                    }
                }
                if let Some((protocol_type, download)) = self.auto_file_transfer.try_transfer(ch) {
                    self.initiate_file_transfer(protocol_type, download);
                    return Ok(());
                }
            }
        }
        if !send_data.is_empty() {
            // println!("Sending: {:?}", String::from_utf8_lossy(&send_data).replace('\x1B', "\\x1B"));
            con.send(send_data)?;
        }

        if con.is_disconnected() {
            self.connection_opt = None;
        }
        self.auto_login.disabled |= self.is_alt_pressed;
        if let Some(adr) = self.addresses.get(self.cur_addr) {
            if let Some(con) = &mut self.connection_opt {
                if let Err(err) = self.auto_login.run_autologin(con, adr) {
                    eprintln!("{err}");
                }
            }
        }

        Ok(())
    }

    pub fn hangup(&mut self) {
        self.open_connection_promise = None;
        if let Some(con) = &mut self.connection_opt {
            con.disconnect().unwrap_or_default();
        }
        self.connection_opt = None;
        self.mode = MainWindowMode::ShowPhonebook;
    }

    pub fn send_login(&mut self) {
        let adr = self.addresses.get(self.cur_addr).unwrap();
        let mut cr = [self.buffer_parser.convert_from_unicode('\r') as u8].to_vec();
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
            let res = con.send(data);
            self.handle_result(res, true);
        }
        self.auto_login.logged_in = true;
    }

    fn update_title(&self, frame: &mut eframe::Frame) {
        if let MainWindowMode::ShowPhonebook = self.mode {
            frame.set_window_title(&crate::DEFAULT_TITLE);
        } else {
            let str = if let Some(con) = &self.connection_opt {
                let d = SystemTime::now()
                    .duration_since(con.get_connection_time())
                    .unwrap();
                let sec = d.as_secs();
                let minutes = sec / 60;
                let hours = minutes / 60;
                let cur = &self.addresses[self.cur_addr];
                let t = format!("{:02}:{:02}:{:02}", hours, minutes % 60, sec % 60);
                let s = if cur.system_name.is_empty() {
                    cur.address.clone()
                } else {
                    cur.system_name.clone()
                };

                fl!(
                    crate::LANGUAGE_LOADER,
                    "title-connected",
                    version = crate::VERSION,
                    time = t,
                    name = s
                )
            } else {
                fl!(
                    crate::LANGUAGE_LOADER,
                    "title-offline",
                    version = crate::VERSION
                )
            };
            frame.set_window_title(str.as_str());
        }
    }

    pub(crate) fn show_settings(&mut self, in_phonebook: bool) {
        self.mode = MainWindowMode::ShowSettings(in_phonebook);
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        use egui::FontFamily::Proportional;
        use egui::TextStyle::{Body, Button, Heading, Monospace, Small};

        let mut style: egui::Style = (*ctx.style()).clone();
        style.text_styles = [
            (Heading, FontId::new(24.0, Proportional)),
            (Body, FontId::new(18.0, Proportional)),
            (Monospace, FontId::new(18.0, egui::FontFamily::Monospace)),
            (Button, FontId::new(18.0, Proportional)),
            (Small, FontId::new(14.0, Proportional)),
        ]
        .into();
        ctx.set_style(style);

        self.update_title(frame);

        if self.open_connection_promise.is_some()
            && self
                .open_connection_promise
                .as_ref()
                .unwrap()
                .ready()
                .is_some()
        {
            if let Ok(handle) = self.open_connection_promise.take().unwrap().try_take() {
                match handle {
                    Ok(handle) => {
                        self.open_connection_promise = None;
                        let ctx = ctx.clone();
                        let (tx, rx) = mpsc::channel::<SendData>(32);
                        let (tx2, mut rx2) = mpsc::channel::<SendData>(32);
                        self.connection_opt = Some(Connection::new(rx, tx2));

                        let mut handle = handle;

                        tokio::spawn(async move {
                            let mut done = false;
                            while !done {
                                tokio::select! {
                                    Ok(v) = handle.read_data() => {
                                        if let Err(err) = tx.send(SendData::Data(v)).await {
                                            eprintln!("error while sending: {err}");
                                            done = true;
                                        } else {
                                            ctx.request_repaint();
                                        }
                                    }
                                    result = rx2.recv() => {
                                        match result {
                                            Some(SendData::Data(buf)) => {
                                                if let Err(err) = handle.send(&buf).await {
                                                    eprintln!("{err}");
                                                    done = true;
                                                }
                                            },
                                            Some(SendData::StartTransfer(protocol_type, download, transfer_state, files_opt)) => {
                                            let mut protocol = protocol_type.create();
                                            if let Err(err) = if download {
                                                    protocol.initiate_recv(&mut handle, transfer_state.clone()).await
                                                } else {
                                                    protocol.initiate_send(&mut handle, files_opt.unwrap(), transfer_state.clone()).await
                                                } {
                                                    eprintln!("{err}");
                                                    break;
                                                }
                                                loop {
                                                    tokio::select! {
                                                        v = protocol.update(&mut handle, transfer_state.clone()) => {
                                                            match v {
                                                                Ok(running) => {
                                                                    if !running {
                                                                        break;
                                                                    }
                                                                }
                                                                Err(err) => {
                                                                    eprintln!("Err {err}");
                                                                    break;
                                                                }
                                                            }
                                                        }
                                                        result = rx2.recv() => {
                                                            if let Some(SendData::CancelTransfer) = result {
                                                                protocol.cancel(&mut handle).await.unwrap_or_default();
                                                                eprintln!("Cancel");
                                                                break;
                                                            }
                                                        }
                                                    }
                                                }
                                                tx.send(SendData::EndTransfer).await.unwrap_or_default();
                                            }
                                            Some(SendData::Disconnect) => {
                                                done = true;
                                            }
                                            _ => {}
                                        }
                                    }
                                };
                            }
                            tx.send(SendData::Disconnect).await.unwrap_or_default();
                        });
                    }
                    Err(err) => {
                        self.println(&format!("\n\r{err}")).unwrap();
                    }
                }
            }
        }

        match self.mode {
            MainWindowMode::ShowTerminal | MainWindowMode::ShowPhonebook => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame);
                self.handle_result(res, false);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowSettings(in_phonebook) => {
                if in_phonebook {
                    super::view_phonebook(self, ctx);
                } else {
                    let res = self.update_state();
                    self.update_terminal_window(ctx, frame);
                    self.handle_result(res, false);
                    ctx.request_repaint_after(Duration::from_millis(150));
                }
                super::show_settings(self, ctx, frame);
            }
            MainWindowMode::SelectProtocol(download) => {
                self.update_terminal_window(ctx, frame);
                super::view_selector(self, ctx, frame, download);
            }
            MainWindowMode::FileTransfer(download) => {
                if self.connection_opt.as_mut().unwrap().should_end_transfer() {
                    /*  if guard.1.is_finished {
                        for f in guard.0.get_received_files() {
                            f.save_file_in_downloads(
                                guard.1.recieve_state.as_mut().unwrap(),
                            )
                            .expect("error saving file.");
                        }
                    } else */
                    self.mode = MainWindowMode::ShowTerminal;
                    self.auto_file_transfer.reset();
                }

                self.update_terminal_window(ctx, frame);
                if let Some(a) = &mut self.current_transfer {
                    // self.print_result(&r);
                    if !super::view_filetransfer(ctx, frame, a, download) {
                        self.mode = MainWindowMode::ShowTerminal;
                        let res = self.connection_opt.as_mut().unwrap().cancel_transfer();
                        self.handle_result(res, true);
                    }
                } else {
                    eprintln!("error - in file transfer but no current protocol.");
                    self.mode = MainWindowMode::ShowTerminal;
                }
                ctx.request_repaint_after(Duration::from_millis(150));
            } // MainWindowMode::AskDeleteEntry => todo!(),
        }
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.buffer_view.lock().destroy(gl);
        }
    }
}
