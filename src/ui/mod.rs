#![allow(unsafe_code, clippy::wildcard_imports)]

use chrono::{Duration, Utc};
use egui::Vec2;
use egui_bind::BindTarget;
use i18n_embed_fl::fl;
use icy_engine::{BufferParser, Position};
use icy_engine_egui::BufferView;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use eframe::egui::Key;

use crate::features::{AutoFileTransfer, AutoLogin};
use crate::protocol::TransferState;
use crate::util::SoundThread;
use crate::Options;
use crate::{protocol::FileDescriptor, TerminalResult};

pub mod app;
pub mod connection;

pub mod terminal_window;
pub use terminal_window::*;

pub mod util;
pub use util::*;

use self::connection::Connection;
pub mod dialogs;

pub mod com_thread;

#[macro_export]
macro_rules! check_error {
    ($main_window: expr, $res: expr, $terminate_connection: expr) => {{
        if let Err(err) = $res {
            log::error!("{err}");
            $main_window.output_string(format!("\n\r{err}\n\r").as_str());

            if $terminate_connection {
                $main_window
                    .connection
                    .as_ref()
                    .unwrap()
                    .disconnect()
                    .unwrap_or_default();
            }
        }
    }};
}

#[derive(PartialEq, Eq)]
pub enum MainWindowMode {
    ShowTerminal,
    ShowDialingDirectory,

    ///Shows settings - parameter: show dialing_directory
    ShowSettings(bool),
    SelectProtocol(bool),
    FileTransfer(bool),
    DeleteSelectedAddress(usize),
    ShowCaptureDialog,
    ShowExportDialog,
    ShowUploadDialog,
    ShowIEMSI, //   AskDeleteEntry
}

pub struct FileTransferState {
    pub current_transfer: Arc<Mutex<TransferState>>,
    pub file_transfer_dialog: dialogs::up_download_dialog::FileTransferDialog,

    pub join_handle: Option<JoinHandle<Box<Connection>>>,
}

impl FileTransferState {
    fn new(
        mut connection: Box<Connection>,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        files_opt: Option<Vec<FileDescriptor>>,
    ) -> Self {
        let current_transfer = Arc::new(Mutex::new(TransferState::default()));

        let current_transfer2 = current_transfer.clone();

        let join_handle = thread::spawn(move || {
            let mut protocol = protocol_type.create();

            if let Err(err) = if download {
                protocol.initiate_recv(&mut connection, &mut current_transfer2.lock().unwrap())
            } else {
                protocol.initiate_send(
                    &mut connection,
                    files_opt.unwrap(),
                    &mut current_transfer2.lock().unwrap(),
                )
            } {
                log::error!("{err}");
                return connection;
            }

            if let Ok(mut storage_handler) = crate::protocol::DiskStorageHandler::new() {
                let mut is_running = true;
                while is_running {
                    if let Err(err) = connection.update_state() {
                        log::error!("Error updating state on file transfer thread: {err}");
                        break;
                    }
                    match protocol.update(&mut connection, &current_transfer2, &mut storage_handler)
                    {
                        Ok(b) => is_running &= b,
                        Err(err) => {
                            log::error!("Error updating protocol on file transfer thread: {err}");
                            break;
                        }
                    }
                    match current_transfer2.lock() {
                        Ok(ct) => {
                            if ct.request_cancel {
                                if let Err(err) = protocol.cancel(&mut connection) {
                                    log::error!(
                                        "Error sending cancel request on file transfer thread: {err}"
                                    );
                                }
                                break;
                            }
                        }
                        Err(err) => {
                            log::error!(
                                "Error locking current_transfer on file transfer thread: {err}"
                            );
                            break;
                        }
                    }
                }
            }
            connection
        });

        Self {
            current_transfer,
            file_transfer_dialog: dialogs::up_download_dialog::FileTransferDialog::new(),
            join_handle: Some(join_handle),
        }
    }
}

pub struct MainWindow {
    buffer_view: Arc<eframe::epaint::mutex::Mutex<BufferView>>,
    pub buffer_parser: Box<dyn BufferParser>,

    connection: Option<Box<Connection>>,

    sound_thread: SoundThread,

    pub mode: MainWindowMode,
    pub handled_char: bool,

    pub options: Options,
    screen_mode: ScreenMode,
    auto_login: AutoLogin,
    is_fullscreen_mode: bool,
    drag_start: Option<Vec2>,
    last_pos: Position,

    /// debug spew prevention
    show_capture_error: bool,

    auto_file_transfer: AutoFileTransfer,

    // protocols
    pub current_file_transfer: Option<FileTransferState>,

    pub dialing_directory_dialog: dialogs::dialing_directory_dialog::DialogState,
    pub capture_dialog: dialogs::capture_dialog::DialogState,
    pub export_dialog: dialogs::export_dialog::DialogState,
    pub upload_dialog: dialogs::upload_dialog::DialogState,
    pub settings_dialog: dialogs::settings_dialog::DialogState,

    #[cfg(target_arch = "wasm32")]
    poll_thread: com_thread::ConnectionThreadData,
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
    pub fn output_char(&mut self, ch: char) {
        let translated_char = self.buffer_parser.convert_from_unicode(ch);
        if self.connection.as_ref().unwrap().is_connected() {
            let r = self
                .connection
                .as_mut()
                .unwrap()
                .send(vec![translated_char as u8]);
            check_error!(self, r, false);
        } else if let Err(err) = self.print_char(translated_char as u8) {
            log::error!("{err}");
        }
    }

    pub fn output_string(&mut self, str: &str) {
        if self.connection.as_ref().unwrap().is_connected() {
            let mut v = Vec::new();
            for ch in str.chars() {
                let translated_char = self.buffer_parser.convert_from_unicode(ch);
                v.push(translated_char as u8);
            }
            let r = self.connection.as_mut().unwrap().send(v);
            check_error!(self, r, false);
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
                if self.connection.as_ref().unwrap().is_connected() {
                    let r = self
                        .connection
                        .as_mut()
                        .unwrap()
                        .send(result.as_bytes().to_vec());
                    check_error!(self, r, false);
                }
            }
            icy_engine::CallbackAction::PlayMusic(music) => {
                let r = self.sound_thread.play_music(music);
                check_error!(self, r, false);
            }
            icy_engine::CallbackAction::Beep => {
                if self.options.console_beep {
                    let r = self.sound_thread.beep();
                    check_error!(self, r, false);
                }
            }
            icy_engine::CallbackAction::ChangeBaudEmulation(baud_emulation) => {
                let r = self
                    .connection
                    .as_mut()
                    .unwrap()
                    .set_baud_rate(baud_emulation.get_baud_rate());
                check_error!(self, r, false);
            }
            icy_engine::CallbackAction::ResizeTerminal(_, _) => {
                self.buffer_view.lock().redraw_view();
            }
        }
        self.buffer_view.lock().redraw_view();
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]

    fn start_file_transfer(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        files_opt: Option<Vec<FileDescriptor>>,
    ) {
        // TODO
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn start_file_transfer(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
        files_opt: Option<Vec<FileDescriptor>>,
    ) {
        self.mode = MainWindowMode::FileTransfer(download);

        let r = crate::protocol::DiskStorageHandler::new();
        check_error!(self, r, false);
        if let Some(mut con) = self.connection.take() {
            con.start_transfer();
            self.current_file_transfer = Some(FileTransferState::new(
                con,
                protocol_type,
                download,
                files_opt,
            ));
        }
    }

    pub(crate) fn initiate_file_transfer(
        &mut self,
        protocol_type: crate::protocol::TransferType,
        download: bool,
    ) {
        self.mode = MainWindowMode::ShowTerminal;
        if self.connection.as_ref().unwrap().is_disconnected() {
            return;
        }

        if download {
            self.start_file_transfer(protocol_type, download, None);
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            self.init_upload_dialog(protocol_type);
        }
    }

    pub fn set_screen_mode(&mut self, mode: ScreenMode) {
        self.screen_mode = mode;
        mode.set_mode(self);
    }

    pub fn show_terminal(&mut self) {
        self.mode = MainWindowMode::ShowTerminal;
    }

    pub fn show_dialing_directory(&mut self) {
        self.mode = MainWindowMode::ShowDialingDirectory;
    }

    pub fn call_bbs_uuid(&mut self, uuid: Option<usize>) {
        if uuid.is_none() {
            self.call_bbs(0);
            return;
        }

        let uuid = uuid.unwrap();
        for (i, adr) in self
            .dialing_directory_dialog
            .addresses
            .addresses
            .iter()
            .enumerate()
        {
            if adr.id == uuid {
                self.call_bbs(i);
                return;
            }
        }
    }

    pub fn call_bbs(&mut self, i: usize) {
        self.mode = MainWindowMode::ShowTerminal;
        let cloned_addr = self.dialing_directory_dialog.addresses.addresses[i].clone();

        {
            let address = &mut self.dialing_directory_dialog.addresses.addresses[i];
            let mut adr = address.address.clone();
            if !adr.contains(':') {
                adr.push_str(":23");
            }
            address.number_of_calls += 1;
            address.last_call = Some(Utc::now());

            self.auto_login = AutoLogin::new(&cloned_addr.auto_login);
            self.buffer_view.lock().buf.clear();
            self.dialing_directory_dialog.cur_addr = i;
            self.buffer_parser = address.get_terminal_parser(&cloned_addr);
            self.buffer_view
                .lock()
                .buf
                .terminal_state
                .set_baud_rate(address.baud_emulation);

            self.buffer_view.lock().redraw_font();
            self.buffer_view.lock().redraw_palette();
            self.buffer_view.lock().redraw_view();
            self.buffer_view.lock().clear();
        }
        self.set_screen_mode(cloned_addr.screen_mode);
        let r = self.dialing_directory_dialog.addresses.store_phone_book();
        check_error!(self, r, false);

        self.println(&fl!(
            crate::LANGUAGE_LOADER,
            "connect-to",
            address = cloned_addr.address.clone()
        ))
        .unwrap_or_default();

        let timeout = self.options.connect_timeout;
        let window_size = self.screen_mode.get_window_size();
        let r = self
            .connection
            .as_mut()
            .unwrap()
            .connect(&cloned_addr, timeout, window_size);
        check_error!(self, r, false);
        let r = self
            .connection
            .as_mut()
            .unwrap()
            .set_baud_rate(cloned_addr.baud_emulation.get_baud_rate());
        check_error!(self, r, false);
    }

    pub fn update_state(&mut self) -> TerminalResult<()> {
        #[cfg(target_arch = "wasm32")]
        self.poll_thread.poll();

        let r = self.connection.as_mut().unwrap().update_state();
        check_error!(self, r, false);
        let r = self.sound_thread.update_state();
        check_error!(self, r, false);
        if self.connection.as_ref().unwrap().is_disconnected() {
            return Ok(());
        }
        let data_opt = if self.connection.as_mut().unwrap().is_data_available()? {
            Some(self.connection.as_mut().unwrap().read_buffer())
        } else {
            None
        };

        if let Some(data) = data_opt {
            if self.capture_dialog.capture_session {
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
                if self.options.iemsi_autologin && self.connection.as_ref().unwrap().is_connected()
                {
                    if let Some(adr) = self
                        .dialing_directory_dialog
                        .addresses
                        .addresses
                        .get(self.dialing_directory_dialog.cur_addr)
                    {
                        if let Err(err) = self.auto_login.try_login(
                            &mut self.connection.as_mut().unwrap(),
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

        if self.options.iemsi_autologin {
            if let Some(adr) = self
                .dialing_directory_dialog
                .addresses
                .addresses
                .get(self.dialing_directory_dialog.cur_addr)
            {
                if self.connection.as_ref().unwrap().is_connected() {
                    if let Err(err) = self
                        .auto_login
                        .run_autologin(&mut self.connection.as_mut().unwrap(), adr)
                    {
                        log::error!("{err}");
                    }
                }
            }
        }

        Ok(())
    }

    pub fn hangup(&mut self) {
        check_error!(self, self.connection.as_ref().unwrap().disconnect(), false);
        self.sound_thread.clear();
        self.mode = MainWindowMode::ShowDialingDirectory;
    }

    pub fn send_login(&mut self) {
        if self.connection.as_ref().unwrap().is_disconnected() {
            return;
        }
        let user_name = self
            .dialing_directory_dialog
            .addresses
            .addresses
            .get(self.dialing_directory_dialog.cur_addr)
            .unwrap()
            .user_name
            .clone();
        let password = self
            .dialing_directory_dialog
            .addresses
            .addresses
            .get(self.dialing_directory_dialog.cur_addr)
            .unwrap()
            .password
            .clone();
        let mut cr: Vec<u8> = [self.buffer_parser.convert_from_unicode('\r') as u8].to_vec();
        for (k, v) in self.screen_mode.get_input_mode().cur_map() {
            if *k == Key::Enter as u32 {
                cr = v.to_vec();
                break;
            }
        }
        self.output_string(&user_name);
        let r = self.connection.as_mut().unwrap().send(cr.clone());
        check_error!(self, r, false);
        self.output_string(&password);
        let r = self.connection.as_mut().unwrap().send(cr);
        check_error!(self, r, false);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_title(&self, frame: &mut eframe::Frame) {
        if let MainWindowMode::ShowDialingDirectory = self.mode {
            frame.set_window_title(&crate::DEFAULT_TITLE);
        } else {
            if self.connection.is_none() {
                return;
            }
            let str = if self.connection.as_ref().unwrap().is_connected() {
                let d = Instant::now()
                    .duration_since(self.connection.as_ref().unwrap().get_connection_time());
                let sec = d.as_secs();
                let minutes = sec / 60;
                let hours = minutes / 60;
                let cur = &self.dialing_directory_dialog.addresses.addresses
                    [self.dialing_directory_dialog.cur_addr];
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

    pub(crate) fn show_settings(&mut self, in_dialing_directory: bool) {
        self.mode = MainWindowMode::ShowSettings(in_dialing_directory);
    }

    fn handle_terminal_key_binds(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if self.options.bind.clear_screen.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.buffer_view.lock().clear_buffer_screen();
        }
        if self.options.bind.dialing_directory.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.mode = MainWindowMode::ShowDialingDirectory;
        }
        if self.options.bind.hangup.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.hangup();
        }
        if self.options.bind.send_login_pw.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.send_login();
        }
        if self.options.bind.show_settings.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.mode = MainWindowMode::ShowSettings(false);
        }
        if self.options.bind.show_capture.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.mode = MainWindowMode::ShowCaptureDialog;
        }
        if self.options.bind.quit.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            #[cfg(not(target_arch = "wasm32"))]
            frame.close();
        }
        if self.options.bind.full_screen.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.is_fullscreen_mode = !self.is_fullscreen_mode;
            #[cfg(not(target_arch = "wasm32"))]
            frame.set_fullscreen(self.is_fullscreen_mode);
        }
        if self.options.bind.upload.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.mode = MainWindowMode::SelectProtocol(false);
        }
        if self.options.bind.download.pressed(ctx) {
            ctx.input_mut(|i| i.events.clear());
            self.mode = MainWindowMode::SelectProtocol(true);
        }
    }
}
