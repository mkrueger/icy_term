use crate::{
    features::{AutoFileTransfer, AutoLogin},
    protocol::TransferType,
    util::SoundThread,
    TerminalResult,
};
use egui::mutex::Mutex;
use icy_engine_egui::BufferView;
use std::{collections::VecDeque, sync::Arc, thread};
use web_time::{Duration, Instant};

use super::{
    connect::{Connection, DataConnection},
    dialogs,
};

pub struct BufferUpdateThread {
    pub capture_dialog: dialogs::capture_dialog::DialogState,

    pub buffer_view: Arc<Mutex<BufferView>>,
    pub connection: Arc<Mutex<Option<Box<Connection>>>>,

    pub last_update: Instant,
    pub auto_file_transfer: AutoFileTransfer,
    pub auto_login: Option<AutoLogin>,
    pub sound_thread: Arc<Mutex<SoundThread>>,

    pub auto_transfer: Option<(TransferType, bool)>,
}

impl BufferUpdateThread {
    pub fn update_state(&mut self, ctx: &egui::Context) -> TerminalResult<bool> {
        let r: Result<_, _> = self.sound_thread.lock().update_state();
        //        check_error!(self, r, false);

        let data = if let Some(con) = self.connection.lock().as_mut() {
            con.update_state()?;
            if con.is_disconnected() {
                return Ok(false);
            }
            if con.is_data_available()? {
                con.read_buffer()
            } else {
                VecDeque::new()
            }
        } else {
            return Ok(false);
        };
        Ok(self.update_buffer(ctx, data))
    }

    fn update_buffer(&mut self, ctx: &egui::Context, mut data: std::collections::VecDeque<u8>) -> bool {
        self.capture_dialog.append_data(&mut data);
        let has_data = !data.is_empty();
        let mut set_buffer_dirty = false;
        if !data.is_empty() {
            println!("data : {}", self.last_update.elapsed().as_millis());
        }
        let buffer_view = &mut self.buffer_view.lock();

        while !data.is_empty() {
            let ch = data.pop_front().unwrap();
            if let Some(autologin) = &mut self.auto_login {
                if let Some(con) = self.connection.lock().as_mut() {
                    if let Err(err) = autologin.try_login(con, ch) {
                        log::error!("{err}");
                    }
                    if autologin.logged_in {
                        self.auto_login = None;
                    }
                }
            }

            if self.print_char(buffer_view, ch) {
                set_buffer_dirty = true;
            }
            if let Some((protocol_type, download)) = self.auto_file_transfer.try_transfer(ch) {
                self.auto_transfer = Some((protocol_type, download));
            }
        }
        if has_data {
            buffer_view.get_buffer_mut().update_hyperlinks();
        }
        if set_buffer_dirty {
            self.last_update = Instant::now();
            buffer_view.get_edit_state_mut().is_buffer_dirty = true;
            return false;
        }
        /*
            if self.get_options().iemsi.autologin {
            if let Some(adr) = self.dialing_directory_dialog.addresses.addresses.get(self.dialing_directory_dialog.cur_addr) {
                if let Some(con) = &mut self.buffer_update_thread.lock().connection {
                    if con.is_connected() {
                        if let Err(err) = self.auto_login.run_autologin(con, adr) {
                            log::error!("{err}");
                        }
                    }
                }
            }
        }*/

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

        true
    }

    pub fn print_char(&self, buffer_view: &mut BufferView, c: u8) -> bool {
        let result = buffer_view.print_char(c as char);
        match result {
            Ok(icy_engine::CallbackAction::SendString(result)) => {
                if let Some(con) = self.connection.lock().as_mut() {
                    if con.is_connected() {
                        let r = con.send(result.as_bytes().to_vec());
                        // check_error!(self, r, false);
                    }
                }
            }
            Ok(icy_engine::CallbackAction::PlayMusic(music)) => {
                let r = self.sound_thread.lock().play_music(music);
                //    check_error!(self, r, false);
            }
            Ok(icy_engine::CallbackAction::Beep) => {
                let r = self.sound_thread.lock().beep();
                //  check_error!(self, r, false);
            }
            Ok(icy_engine::CallbackAction::ChangeBaudEmulation(baud_emulation)) => {
                if let Some(con) = self.connection.lock().as_mut() {
                    let r = con.set_baud_rate(baud_emulation.get_baud_rate());
                    //    check_error!(self, r, false);
                }
            }
            Ok(icy_engine::CallbackAction::ResizeTerminal(_, _)) => {
                buffer_view.redraw_view();
            }

            Ok(icy_engine::CallbackAction::NoUpdate) => {
                return false;
            }

            Ok(icy_engine::CallbackAction::Update) => {
                return true;
            }

            Err(err) => {
                log::error!("{err}");
            }
        }
        false
    }
}

pub fn run_update_thread(ctx: &egui::Context, update_thread: Arc<Mutex<BufferUpdateThread>>) {
    let ctx = ctx.clone();
    thread::spawn(move || loop {
        match update_thread.lock().update_state(&ctx) {
            Err(err) => {
                log::error!("{err}");
            }
            Ok(sleep) => {
                if sleep {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    });
}
