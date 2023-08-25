#![allow(unsafe_code, clippy::wildcard_imports)]

use core::panic;
use std::{sync::Arc, time::Duration};

use eframe::egui::{self};
use egui::FontId;
use icy_engine::{ansi, Position};
use web_time::Instant;

use crate::{
    check_error,
    features::{AutoFileTransfer, AutoLogin},
    ui::{
        dialogs::{self, capture_dialog},
        BufferView, ScreenMode,
    },
    util::SoundThread,
    AddressBook, Options,
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
                log::error!("Error reading dialing_directory: {e}");
                Options::default()
            }
        };

        let view = BufferView::new(gl, options.scaling.get_filter());

        let addresses = match crate::addresses::start_read_book() {
            Ok(addresses) => addresses,
            Err(e) => {
                log::error!("Error reading dialing_directory: {e}");
                AddressBook::default()
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let connection = MainWindow::start_com_thread();
        #[cfg(target_arch = "wasm32")]
        let (connection, poll_thread) = MainWindow::start_poll_thead();
        #[cfg(not(target_arch = "wasm32"))]
        let is_fullscreen_mode = cc.integration_info.window_info.fullscreen;
        #[cfg(target_arch = "wasm32")]
        let is_fullscreen_mode = false;
        let mut view = MainWindow {
            buffer_view: Arc::new(eframe::epaint::mutex::Mutex::new(view)),
            //address_list: HoverList::new(),
            mode: MainWindowMode::ShowDialingDirectory,
            connection: Some(Box::new(connection)),
            options,
            auto_login: AutoLogin::new(""),
            auto_file_transfer: AutoFileTransfer::default(),
            screen_mode: ScreenMode::default(),
            current_file_transfer: None,
            handled_char: false,
            buffer_parser: Box::<ansi::Parser>::default(),
            show_capture_error: false,
            #[cfg(target_arch = "wasm32")]
            poll_thread,
            sound_thread: SoundThread::new(),
            is_fullscreen_mode,
            capture_dialog: capture_dialog::DialogState::default(),
            export_dialog: dialogs::export_dialog::DialogState::default(),
            upload_dialog: dialogs::upload_dialog::DialogState::default(),
            dialing_directory_dialog: dialogs::dialing_directory_dialog::DialogState::new(
                addresses,
            ),
            settings_dialog: dialogs::settings_dialog::DialogState::default(),
            drag_start: None,
            last_pos: Position::default(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        parse_command_line(&mut view);

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

    fn get_connection_back(&mut self) {
        if let Some(fts) = &mut self.current_file_transfer {
            if let Some(handle) = fts.join_handle.take() {
                if let Ok(join) = handle.join() {
                    self.connection = Some(join);
                    self.mode = MainWindowMode::ShowTerminal;
                } else {
                    panic!("Error joining file transfer thread.");
                }
            } else {
                panic!("Error joining file transfer thread - no join handle.");
            }
        } else {
            panic!("Error joining file transfer thread - no current file transfer.");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_command_line(view: &mut MainWindow) {
    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        view.dialing_directory_dialog.addresses.addresses[0].address = arg.clone();
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
            MainWindowMode::ShowDialingDirectory => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, true);
                check_error!(self, res, false);
            }
            MainWindowMode::ShowSettings(in_dialing_directory) => {
                if in_dialing_directory {
                    dialogs::dialing_directory_dialog::view_dialing_directory(self, ctx);
                } else {
                    let res = self.update_state();
                    self.update_terminal_window(ctx, frame, false);
                    check_error!(self, res, false);
                    ctx.request_repaint_after(Duration::from_millis(150));
                }
                dialogs::settings_dialog::show_settings(self, ctx, frame);
            }
            MainWindowMode::DeleteSelectedAddress(uuid) => {
                self.update_terminal_window(ctx, frame, true);
                super::dialogs::show_delete_address_confirmation::show_dialog(self, ctx, uuid);
            }

            MainWindowMode::SelectProtocol(download) => {
                self.update_terminal_window(ctx, frame, false);
                dialogs::protocol_selector::view_selector(self, ctx, frame, download);
            }

            MainWindowMode::FileTransfer(download) => {
                self.update_terminal_window(ctx, frame, false);

                let mut join_thread = false;
                if let Some(fts) = &mut self.current_file_transfer {
                    let state = if let Ok(state) = fts.current_transfer.lock() {
                        Some(state.clone())
                    } else {
                        log::error!("In file transfer but can't lock state.");
                        join_thread = true;
                        None
                    };

                    if let Some(state) = state {
                        if state.is_finished {
                            join_thread = true;
                        } else if !fts
                            .file_transfer_dialog
                            .show_dialog(ctx, frame, &state, download)
                        {
                            fts.current_transfer.lock().unwrap().request_cancel = true;
                            join_thread = true;
                        }
                    }
                } else {
                    log::error!("In file transfer but no current protocol.");
                    join_thread = true;
                }
                if join_thread {
                    self.get_connection_back();
                }
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowCaptureDialog => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, false);
                check_error!(self, res, false);
                self.show_caputure_dialog(ctx);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowExportDialog => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, false);
                check_error!(self, res, false);
                self.show_export_dialog(ctx);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowUploadDialog => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, false);
                check_error!(self, res, false);
                self.show_upload_dialog(ctx);
                ctx.request_repaint_after(Duration::from_millis(150));
            }
            MainWindowMode::ShowIEMSI => {
                let res = self.update_state();
                self.update_terminal_window(ctx, frame, false);
                check_error!(self, res, false);
                dialogs::show_iemsi::show_iemsi(self, ctx);
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
