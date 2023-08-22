#![allow(unsafe_code, clippy::wildcard_imports)]

use std::{sync::Arc, time::Duration};

use eframe::egui::{self};
use egui::FontId;
use icy_engine::ansi;

use crate::{
    check_error,
    features::{AutoFileTransfer, AutoLogin},
    ui::{dialogs::PhonebookFilter, BufferView, ScreenMode},
    util::{Rng, SoundThread},
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
        #[cfg(not(target_arch = "wasm32"))]
        let connection = MainWindow::start_com_thread();
        #[cfg(target_arch = "wasm32")]
        let (connection, poll_thread) = MainWindow::start_poll_thead();

        let mut view = MainWindow {
            buffer_view: Arc::new(eframe::epaint::mutex::Mutex::new(view)),
            //address_list: HoverList::new(),
            mode: MainWindowMode::ShowPhonebook,
            addresses,
            cur_addr: 0,
            selected_bbs: None,
            connection,
            options,
            auto_login: AutoLogin::new(""),
            auto_file_transfer: AutoFileTransfer::default(),
            screen_mode: ScreenMode::Vga(80, 25),
            current_transfer: None,
            handled_char: false,
            is_alt_pressed: false,
            phonebook_filter: PhonebookFilter::All,
            buffer_parser: Box::<ansi::Parser>::default(),
            phonebook_filter_string: String::new(),
            scroll_address_list_to_bottom: false,
            rng: Rng::default(),
            capture_session: false,
            show_capture_error: false,
            has_baud_rate: false,
            settings_category: 0,
            file_transfer_dialog: crate::ui::dialogs::FileTransferDialog::default(),
            #[cfg(target_arch = "wasm32")]
            poll_thread,
            sound_thread: SoundThread::new(),
            is_fullscreen_mode: cc.integration_info.window_info.fullscreen,
        };

        #[cfg(not(target_arch = "wasm32"))]
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

        // try to detect dark vs light mode from the host system; default to dark
        ctx.set_visuals(if dark_light::detect() == dark_light::Mode::Light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        });

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

        match self.mode {
            MainWindowMode::ShowTerminal => {
                let res = self.update_state();
                self.handle_terminal_key_binds(ctx, frame);
                self.update_terminal_window(ctx, frame, false);
                check_error!(self, res, false);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowPhonebook => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, true);
                check_error!(self, res, false);
            }
            MainWindowMode::ShowSettings(in_phonebook) => {
                if in_phonebook {
                    super::dialogs::view_phonebook(self, ctx);
                } else {
                    let res = self.update_state();
                    self.update_terminal_window(ctx, frame, false);
                    check_error!(self, res, false);
                    ctx.request_repaint_after(Duration::from_millis(150));
                }
                super::dialogs::show_settings(self, ctx, frame);
            }
            MainWindowMode::DeleteSelectedAddress(uuid) => {
                self.update_terminal_window(ctx, frame, true);
                super::dialogs::show_delete_address_confirmation::show_dialog(self, ctx, uuid);
            }

            MainWindowMode::SelectProtocol(download) => {
                self.update_terminal_window(ctx, frame, false);
                super::dialogs::view_selector(self, ctx, frame, download);
            }

            MainWindowMode::FileTransfer(download) => {
                if self.connection.should_end_transfer() {
                    self.auto_file_transfer.reset();
                }

                self.update_terminal_window(ctx, frame, false);
                if let Some(a) = &mut self.current_transfer {
                    let state = {
                        let Ok(state) = a.lock() else {
                            log::error!("In file transfer but can't lock state.");
                            self.mode = MainWindowMode::ShowTerminal;
                            return;
                        };
                        state.clone()
                    };
                    if state.is_finished {
                        self.mode = MainWindowMode::ShowTerminal;
                    }
                    if !self
                        .file_transfer_dialog
                        .show_dialog(ctx, frame, &state, download)
                    {
                        self.mode = MainWindowMode::ShowTerminal;
                        let res = self.connection.cancel_transfer();
                        check_error!(self, res, true);
                    }
                } else {
                    log::error!("In file transfer but no current protocol.");
                    self.mode = MainWindowMode::ShowTerminal;
                }
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowCaptureDialog => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, false);
                check_error!(self, res, false);
                #[cfg(not(target_arch = "wasm32"))]
                super::dialogs::show_dialog(self, ctx);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowIEMSI => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, false);
                check_error!(self, res, false);
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
