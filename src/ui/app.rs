#![allow(unsafe_code, clippy::wildcard_imports)]

use std::{sync::Arc, time::Duration};

use eframe::egui::{self};
use egui::FontId;
use icy_engine::ansi;

use crate::{
    features::{AutoFileTransfer, AutoLogin},
    protocol::TransferState,
    ui::{dialogs::PhonebookFilter, BufferView, ScreenMode},
    util::Rng,
    Options,
};

use super::{MainWindow, MainWindowMode};

impl MainWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        use egui::FontFamily::Proportional;
        use egui::TextStyle::{Body, Button, Heading, Monospace, Small};

        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        let options = match Options::load_options() {
            Ok(options) => options,
            Err(e) => {
                log::error!("Error reading phonebook: {e}");
                Options::default()
            }
        };

        let view = BufferView::new(gl, &options);

        let addresses = match crate::addresses::start_read_book() {
            Ok(addresses) => addresses,
            Err(e) => {
                log::error!("Error reading phonebook: {e}");
                vec![crate::Address::new(String::new())]
            }
        };

        let mut view = MainWindow {
            buffer_view: Arc::new(eframe::epaint::mutex::Mutex::new(view)),
            //address_list: HoverList::new(),
            mode: MainWindowMode::ShowPhonebook,
            addresses,
            cur_addr: 0,
            selected_bbs: None,
            connection_opt: None,
            options,
            auto_login: AutoLogin::new(""),
            auto_file_transfer: AutoFileTransfer::default(),
            screen_mode: ScreenMode::Vga(80, 25),
            current_transfer: None,
            handled_char: false,
            is_alt_pressed: false,
            phonebook_filter: PhonebookFilter::All,
            buffer_parser: Box::<ansi::Parser>::default(),
            open_connection_promise: None,
            phonebook_filter_string: String::new(),
            rng: Rng::default(),
            capture_session: false,
            show_capture_error: false,
            settings_category: 0,
            file_transfer_dialog: crate::ui::dialogs::FileTransferDialog::default(),
        };

        parse_command_line(&mut view);

        //view.address_list.selected_item = 1;
        // view.set_screen_mode(&ScreenMode::Viewdata);
        //view.update_address_list();
        /*
        unsafe {
            view.mode = MainWindowMode::ShowTerminal;
            super::simulate::run_sim(&mut view);
        }*/
        /*
                view.mode = MainWindowMode::FileTransfer(true);

                let mut transfer = TransferState::default();

                {}
                transfer.recieve_state.log_info("Hello World");
                transfer.recieve_state.log_warning("Hello World");
                transfer.recieve_state.log_error("Hello World");
                view.current_transfer = Some(Arc::new(std::sync::Mutex::new(transfer)));
        */
        let ctx = &cc.egui_ctx;

        let mut style: egui::Style = (*ctx.style()).clone();
        style.spacing.window_margin = egui::Margin::same(8.0);
        //        style.spacing.button_padding = Vec2::new(4., 2.);
        style.text_styles = [
            (Heading, FontId::new(24.0, Proportional)),
            (Body, FontId::new(18.0, Proportional)),
            (Monospace, FontId::new(18.0, egui::FontFamily::Monospace)),
            (Button, FontId::new(18.0, Proportional)),
            (Small, FontId::new(14.0, Proportional)),
        ]
        .into();
        ctx.set_style(style);

        view
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_command_line(view: &mut MainWindow) {
    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        view.addresses[0].address = arg.clone();
        view.call_bbs(0);
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        self.update_title(frame);

        if self.open_connection_promise.is_some()
            && self.open_connection_promise.as_ref().unwrap().is_finished()
        {
            if let Some(join_handle) = self.open_connection_promise.take() {
                let handle = join_handle.join();
                if let Ok(handle) = handle {
                    match handle {
                        Ok(handle) => {
                            self.open_connection(ctx, handle);
                        }
                        Err(err) => {
                            self.println(&format!("\n\r{err}")).unwrap();
                        }
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
                    super::dialogs::view_phonebook(self, ctx);
                } else {
                    let res = self.update_state();
                    self.update_terminal_window(ctx, frame);
                    self.handle_result(res, false);
                    ctx.request_repaint_after(Duration::from_millis(150));
                }
                super::dialogs::show_settings(self, ctx, frame);
            }
            MainWindowMode::SelectProtocol(download) => {
                self.update_terminal_window(ctx, frame);
                super::dialogs::view_selector(self, ctx, frame, download);
            }

            MainWindowMode::FileTransfer(download) => {
                if let Some(con) = self.connection_opt.as_mut() {
                    if con.should_end_transfer() {
                        self.auto_file_transfer.reset();
                    }
                }

                self.update_terminal_window(ctx, frame);
                if let Some(a) = &mut self.current_transfer {
                    let state = {
                        let Ok(state) = a.lock() else {
                            log::error!("In file transfer but can't lock state.");
                            self.mode = MainWindowMode::ShowTerminal;
                            return;
                        };
                        state.clone()
                    };

                    if !self
                        .file_transfer_dialog
                        .show_dialog(ctx, frame, &state, download)
                    {
                        self.mode = MainWindowMode::ShowTerminal;
                        if let Some(con) = self.connection_opt.as_mut() {
                            let res = con.cancel_transfer();
                            self.handle_result(res, true);
                        } else {
                            log::error!("In file transfer but connection lost.");
                            self.mode = MainWindowMode::ShowTerminal;
                        }
                    }
                } else {
                    log::error!("In file transfer but no current protocol.");
                    self.mode = MainWindowMode::ShowTerminal;
                }
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowCaptureDialog => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame);
                self.handle_result(res, false);
                super::dialogs::show_dialog(self, ctx);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowIEMSI => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame);
                self.handle_result(res, false);
                super::dialogs::show_iemsi(self, ctx);
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
