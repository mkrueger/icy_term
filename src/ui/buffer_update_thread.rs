use crate::TerminalResult;
use egui::mutex::Mutex;
use icy_engine_egui::BufferView;
use std::{collections::VecDeque, sync::Arc, thread};
use web_time::Duration;

use super::{
    connect::{Connection, DataConnection},
    dialogs,
};

pub struct BufferUpdateThread {
    pub capture_dialog: dialogs::capture_dialog::DialogState,

    pub buffer_view: Arc<Mutex<BufferView>>,
    pub connection: Arc<Mutex<Option<Box<Connection>>>>,
}

impl BufferUpdateThread {
    pub fn update_state(&mut self, ctx: &egui::Context) -> TerminalResult<()> {
        let data = if let Some(con) = self.connection.lock().as_mut() {
            con.update_state();
            if con.is_disconnected() {
                return Ok(());
            }
            if con.is_data_available()? {
                con.read_buffer()
            } else {
                VecDeque::new()
            }
        } else {
            thread::sleep(Duration::from_millis(100));
            return Ok(());
        };

        self.update_buffer(ctx, data);

        Ok(())
    }

    fn update_buffer(&mut self, ctx: &egui::Context, mut data: std::collections::VecDeque<u8>) {
        self.capture_dialog.append_data(&mut data);
        let has_data = !data.is_empty();
        let mut set_buffer_dirty = false;
        while !data.is_empty() {
            let ch = data.pop_front().unwrap();
            /* if self.get_options().iemsi.autologin && self.connection().is_connected() {
                if let Some(adr) = self.dialing_directory_dialog.addresses.addresses.get(self.dialing_directory_dialog.cur_addr) {
                    if let Some(con) = &mut self.buffer_update_thread.lock().connection {
                        if let Err(err) = self.auto_login.try_login(con, adr, ch, &self.state.options) {
                            log::error!("{err}");
                        }
                    }
                }
            }*/

            if self.print_char(Some(ctx), ch) {
                set_buffer_dirty = true;
            }
            /*
            if let Some((protocol_type, download)) = self.auto_file_transfer.try_transfer(ch) {
                self.initiate_file_transfer(protocol_type, download);
                return Ok(());
            }*/
        }

        if set_buffer_dirty {
            println!("set_buffer_dirty");
            self.buffer_view.lock().get_edit_state_mut().is_buffer_dirty = true;
        }

        if has_data {
            self.buffer_view.lock().get_buffer_mut().update_hyperlinks();
        } else {
            thread::sleep(Duration::from_millis(10));
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
    }

    pub fn print_char(&self, ctx: Option<&egui::Context>, c: u8) -> bool {
        let result = self.buffer_view.lock().print_char(unsafe { char::from_u32_unchecked(c as u32) });
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
                //    let r = self.sound_thread.play_music(music);
                //    check_error!(self, r, false);
            }
            Ok(icy_engine::CallbackAction::Beep) => {
                /*  if self.get_options().console_beep {
                    let r = self.sound_thread.beep();
                  //  check_error!(self, r, false);
                }*/
            }
            Ok(icy_engine::CallbackAction::ChangeBaudEmulation(baud_emulation)) => {
                if let Some(con) = self.connection.lock().as_mut() {
                    let r = con.set_baud_rate(baud_emulation.get_baud_rate());
                    //    check_error!(self, r, false);
                }
            }
            Ok(icy_engine::CallbackAction::ResizeTerminal(_, _)) => {
                self.buffer_view.lock().redraw_view();
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
        if let Err(err) = update_thread.lock().update_state(&ctx) {
            log::error!("{err}");
        }
    });
}
