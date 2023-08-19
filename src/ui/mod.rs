#![allow(unsafe_code, clippy::wildcard_imports)]

use chrono::Utc;
use i18n_embed_fl::fl;
use icy_engine::ansi::BaudEmulation;
use icy_engine::BufferParser;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use eframe::egui::{self, Key};

use crate::com::{Com, TermComResult};
use crate::features::{AutoFileTransfer, AutoLogin};
use crate::protocol::{TestStorageHandler, TransferState};
use crate::util::{beep, play_music, Rng};
use crate::Options;
use crate::{
    addresses::{store_phone_book, Address},
    com::{ComRawImpl, ComTelnetImpl, SendData},
    protocol::FileDescriptor,
    TerminalResult,
};

use crate::com::Connection;

const BITS_PER_BYTE: u32 = 8;

pub mod app;

pub mod buffer_view;
pub use buffer_view::*;

pub mod terminal_window;
pub use terminal_window::*;

pub mod util;
pub use util::*;

pub mod dialogs;

// pub mod simulate;

#[derive(PartialEq, Eq)]
pub enum MainWindowMode {
    ShowTerminal,
    ShowPhonebook,
    ShowSettings(bool),
    SelectProtocol(bool),
    FileTransfer(bool),
    ShowCaptureDialog,
    ShowIEMSI, //   AskDeleteEntry
}

pub struct MainWindow {
    pub buffer_view: Arc<eframe::epaint::mutex::Mutex<BufferView>>,
    pub buffer_parser: Box<dyn BufferParser>,

    pub connection_opt: Option<Connection>,

    pub mode: MainWindowMode,
    pub addresses: Vec<Address>,
    pub handled_char: bool,
    pub cur_addr: usize,
    pub selected_bbs: Option<usize>,
    pub phonebook_filter: dialogs::PhonebookFilter,
    pub phonebook_filter_string: String,

    pub options: Options,
    pub screen_mode: ScreenMode,
    pub auto_login: AutoLogin,
    pub capture_session: bool,
    /// debug spew prevention
    pub show_capture_error: bool,
    pub has_baud_rate: bool,

    pub rng: Rng,
    pub auto_file_transfer: AutoFileTransfer,
    // protocols
    pub current_transfer: Option<Arc<Mutex<TransferState>>>,
    pub is_alt_pressed: bool,

    pub open_connection_promise: Option<JoinHandle<TermComResult<Box<dyn Com>>>>,

    pub settings_category: usize,

    file_transfer_dialog: dialogs::FileTransferDialog,
}

impl MainWindow {
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
            log::error!("{err}");

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
        if let Some(con) = &mut self.connection_opt {
            let r = con.send(vec![translated_char as u8]);
            self.handle_result(r, false);
        } else if let Err(err) = self.print_char(translated_char as u8) {
            log::error!("{err}");
        }
    }

    pub fn output_string(&mut self, str: &str) {
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
                    log::error!("{err}");
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
            icy_engine::CallbackAction::PlayMusic(music) => {
                play_music(&music);
            }
            icy_engine::CallbackAction::Beep => {
                if self.options.console_beep {
                    beep();
                }
            }
            icy_engine::CallbackAction::ChangeBaudEmulation(baud_emulation) => {
                if let Some(con) = &mut self.connection_opt {
                    let r = con.set_baud_rate(baud_emulation.get_baud_rate());
                    self.handle_result(r, false);
                }
            }
        }
        self.buffer_view.lock().redraw_view();
        Ok(())
    }

    fn start_transfer_thread(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        files_opt: Option<Vec<FileDescriptor>>,
    ) {
        self.mode = MainWindowMode::FileTransfer(download);
        let state = Arc::new(Mutex::new(TransferState::default()));
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
                                        log::error!("{}", error);
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
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let files = rfd::FileDialog::new().pick_files();
                        if let Some(path) = files {
                            let fd = FileDescriptor::from_paths(&path);
                            if let Ok(files) = fd {
                                self.start_transfer_thread(protocol_type, download, Some(files));
                            }
                        }
                    }
                }
            }
            None => {
                log::error!("Communication error.");
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
        self.buffer_parser = self.addresses[i].get_terminal_parser(&call_adr);
        self.has_baud_rate = self.addresses[i].baud_emulation != BaudEmulation::Off;

        self.buffer_view
            .lock()
            .buf
            .terminal_state
            .set_baud_rate(self.addresses[i].baud_emulation);

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

        self.open_connection_promise = Some(thread::spawn(move || {
            let mut com: Box<dyn Com> = match ct {
                crate::addresses::Protocol::Telnet => Box::new(ComTelnetImpl::new(window_size)),
                crate::addresses::Protocol::Raw => Box::new(ComRawImpl::new()),
                crate::addresses::Protocol::Ssh => panic!(), //Box::new(crate::com::SSHCom::new()),
            };
            if let Err(err) = com.connect(&call_adr, timeout) {
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
        let data_opt = if let Some(con) = &mut self.connection_opt {
            if con.is_disconnected() {
                self.connection_opt = None;
                return Ok(());
            }
            if con.is_data_available()? {
                Some(con.read_buffer())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(data) = data_opt {
            if self.capture_session {
                if let Ok(mut data_file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.options.capture_filename)
                {
                    if let Err(err) = data_file.write_all(&data) {
                        if !self.show_capture_error {
                            self.show_capture_error = true;
                            log::error!("{err}");
                        }
                    }
                }
            }

            for ch in data {
                if self.options.iemsi_autologin {
                    if let Some(adr) = self.addresses.get(self.cur_addr) {
                        if let Err(err) = self.auto_login.try_login(
                            &mut self.connection_opt,
                            adr,
                            ch,
                            &self.options,
                        ) {
                            log::error!("{err}");
                        }
                    }
                }

                /*
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
                }*/

                if let Err(err) = self.print_char(ch) {
                    log::error!("{err}");
                }

                if let Some((protocol_type, download)) = self.auto_file_transfer.try_transfer(ch) {
                    self.initiate_file_transfer(protocol_type, download);
                    return Ok(());
                }
            }
        }

        self.auto_login.disabled |= self.is_alt_pressed;
        if self.options.iemsi_autologin {
            if let Some(adr) = self.addresses.get(self.cur_addr) {
                if let Some(con) = &mut self.connection_opt {
                    if let Err(err) = self.auto_login.run_autologin(con, adr) {
                        log::error!("{err}");
                    }
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
        let user_name = self.addresses.get(self.cur_addr).unwrap().user_name.clone();
        let password = self.addresses.get(self.cur_addr).unwrap().password.clone();
        let mut cr: Vec<u8> = [self.buffer_parser.convert_from_unicode('\r') as u8].to_vec();
        for (k, v) in self.screen_mode.get_input_mode().cur_map() {
            if *k == Key::Enter as u32 {
                cr = v.to_vec();
                break;
            }
        }
        self.output_string(&user_name);
        if let Some(con) = &mut self.connection_opt {
            let r = con.send(cr.clone());
            self.handle_result(r, false);
        }
        self.output_string(&password);
        if let Some(con) = &mut self.connection_opt {
            let r = con.send(cr);
            self.handle_result(r, false);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_title(&self, frame: &mut eframe::Frame) {
        if let MainWindowMode::ShowPhonebook = self.mode {
            frame.set_window_title(&crate::DEFAULT_TITLE);
        } else {
            let str = if let Some(con) = &self.connection_opt {
                let d = Instant::now().duration_since(con.get_connection_time());
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

    pub fn open_connection(&mut self, ctx: &egui::Context, handle: Box<dyn Com>) {
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel::<SendData>();
        let (tx2, rx2) = mpsc::channel::<SendData>();
        self.connection_opt = Some(Connection::new(rx, tx2));
        let handle: Arc<Mutex<Box<dyn Com>>> = Arc::new(Mutex::new(handle));
        let handle2 = handle.clone();
        let tx3 = tx.clone();
        let mut baud_rate = self
            .buffer_view
            .lock()
            .buf
            .terminal_state
            .get_baud_emulation()
            .get_baud_rate();

        thread::spawn(move || {
            let mut done = false;

            let mut data_buffer = VecDeque::<u8>::new();
            let mut time = Instant::now();

            while !done {
                if data_buffer.is_empty() {
                    if let Ok(Some(data)) = handle2.lock().unwrap().read_data() {
                        if baud_rate == 0 {
                            if let Err(err) = tx3.send(SendData::Data(data)) {
                                log::error!("{err}");
                                done = true;
                            }
                            ctx.request_repaint();
                        } else {
                            data_buffer.extend(data);
                        }
                    } else {
                        thread::sleep(Duration::from_millis(25));
                    }
                } else if baud_rate == 0 {
                    if let Err(err) = tx3.send(SendData::Data(data_buffer.drain(..).collect())) {
                        log::error!("{err}");
                        done = true;
                    }
                    ctx.request_repaint();
                } else {
                    let cur_time = Instant::now();
                    let bytes_per_sec = baud_rate / BITS_PER_BYTE;
                    let elapsed_ms = cur_time.duration_since(time).as_millis() as u32;
                    let bytes_to_send: usize = ((bytes_per_sec * elapsed_ms) / 1000)
                        .min(data_buffer.len() as u32)
                        as usize;

                    if bytes_to_send > 0 {
                        if let Err(err) =
                            tx3.send(SendData::Data(data_buffer.drain(..bytes_to_send).collect()))
                        {
                            log::error!("{err}");
                            done = true;
                        }
                        time = cur_time;
                    }
                }

                while let Ok(result) = rx2.try_recv() {
                    match result {
                        SendData::Data(buf) => {
                            if let Err(err) = handle.lock().unwrap().send(&buf) {
                                log::error!("{err}");
                                done = true;
                            }
                        }
                        SendData::StartTransfer(
                            protocol_type,
                            download,
                            transfer_state,
                            files_opt,
                        ) => {
                            let mut copy_state = transfer_state.lock().unwrap().clone();

                            let mut protocol = protocol_type.create();

                            if let Err(err) = if download {
                                protocol.initiate_recv(&mut handle.lock().unwrap(), &mut copy_state)
                            } else {
                                protocol.initiate_send(
                                    &mut handle.lock().unwrap(),
                                    files_opt.unwrap(),
                                    &mut copy_state,
                                )
                            } {
                                log::error!("{err}");
                                break;
                            }
                            let mut storage_handler: TestStorageHandler = TestStorageHandler::new();

                            loop {
                                let v = protocol.update(
                                    &mut handle.lock().unwrap(),
                                    &mut copy_state,
                                    &mut storage_handler,
                                );
                                match v {
                                    Ok(running) => {
                                        if !running {
                                            break;
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("Error, aborting protocol: {err}");
                                        copy_state.is_finished = true;
                                        break;
                                    }
                                }
                                if let Ok(SendData::CancelTransfer) = rx2.try_recv() {
                                    protocol
                                        .cancel(&mut handle.lock().unwrap())
                                        .unwrap_or_default();
                                    break;
                                }
                                *transfer_state.lock().unwrap() = copy_state.clone();
                            }
                            *transfer_state.lock().unwrap() = copy_state.clone();
                            #[cfg(not(target_arch = "wasm32"))]
                            if let Some(user_dirs) = directories::UserDirs::new() {
                                let dir = user_dirs.download_dir().unwrap();

                                for file in &storage_handler.file {
                                    let f = if file.0.is_empty() {
                                        "new_file".to_string()
                                    } else {
                                        file.0.clone()
                                    };

                                    let mut file_name = dir.join(file.0);
                                    let mut i = 1;
                                    while file_name.exists() {
                                        file_name = dir.join(&format!("{}.{}", f, i));
                                        i += 1;
                                    }
                                    std::fs::write(file_name, file.1.clone()).unwrap_or_default();
                                }
                            }
                            tx.send(SendData::EndTransfer).unwrap_or_default();
                        }
                        SendData::SetBaudRate(baud) => {
                            baud_rate = baud;
                        }
                        SendData::Disconnect => {
                            done = true;
                        }
                        _ => {}
                    }
                }
            }
            tx.send(SendData::Disconnect).unwrap_or_default();
        });
    }
}
