use eframe::egui::{self, RichText};
use egui::TextEdit;
use egui_file::FileDialog;
use i18n_embed_fl::fl;

use crate::ui::{MainWindow, MainWindowMode};

#[derive(Default)]
pub struct DialogState {
    open_file_dialog: Option<FileDialog>,
}

impl MainWindow {
    pub fn show_caputure_dialog(&mut self, ctx: &egui::Context) {
        use std::path::Path;

        use egui::{Frame, Layout};

        use crate::check_error;

        let mut open = true;
        let mut close_dialog = false;
        let mut changed = true;
        if ctx.input(|i: &egui::InputState| i.key_down(egui::Key::Escape)) {
            open = false;
        }
        let window_frame = Frame::window(&ctx.style());
        egui::Window::new(fl!(crate::LANGUAGE_LOADER, "capture-dialog-capture-title"))
            .open(&mut open)
            .collapsible(true)
            .frame(window_frame)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "capture-dialog-capture-label"
                )));

                ui.horizontal(|ui| {
                    let r = ui.add(
                        TextEdit::singleline(&mut self.options.capture_filename)
                            .desired_width(370.),
                    );
                    if r.changed() {
                        changed = true;
                    }
                    if ui.button("…").clicked() {
                        let initial_path = if self.options.capture_filename.is_empty() {
                            None
                        } else {
                            Path::new(&self.options.capture_filename)
                                .parent()
                                .map(std::path::Path::to_path_buf)
                        };
                        let mut dialog: FileDialog = FileDialog::save_file(initial_path);
                        dialog.open();
                        self.capture_dialog.open_file_dialog = Some(dialog);
                    }
                });
                ui.add_space(8.);
                ui.separator();
                ui.add_space(4.0);

                ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                    if self.capture_session {
                        if ui
                            .button(fl!(crate::LANGUAGE_LOADER, "toolbar-stop-capture"))
                            .clicked()
                        {
                            self.capture_session = false;
                            close_dialog = true;
                        }
                    } else if ui
                        .button(fl!(crate::LANGUAGE_LOADER, "capture-dialog-capture-button"))
                        .clicked()
                    {
                        self.capture_session = true;
                        close_dialog = true;
                    }

                    if let Some(path) = Path::new(&self.options.capture_filename).parent() {
                        if ui
                            .button(fl!(
                                crate::LANGUAGE_LOADER,
                                "capture-dialog-open-folder-button"
                            ))
                            .clicked()
                        {
                            if let Some(s) = path.to_str() {
                                if let Err(err) = open::that(s) {
                                    log::error!("Failed to open folder: {}", err);
                                }
                            }
                        }
                    }

                    if ui
                        .button(fl!(
                            crate::LANGUAGE_LOADER,
                            "dialing_directory-cancel-button"
                        ))
                        .clicked()
                    {
                        close_dialog = true;
                    }
                });
            });

        if let Some(dialog) = &mut self.capture_dialog.open_file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(path) = dialog.path() {
                    if let Some(s) = path.to_str() {
                        self.options.capture_filename = s.to_string();
                        changed = true;
                    }
                }
            }
        }

        if changed {
            self.show_capture_error = false;
            check_error!(self, self.options.store_options(), false);
        }

        if !open || close_dialog {
            self.mode = MainWindowMode::ShowTerminal;
        }
    }
}
