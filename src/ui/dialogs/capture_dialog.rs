use eframe::egui::{self, RichText};
use egui::TextEdit;
use i18n_embed_fl::fl;

use crate::ui::{MainWindow, MainWindowMode};

#[cfg(not(target_arch = "wasm32"))]
pub fn show_dialog(window: &mut MainWindow, ctx: &egui::Context) {
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
                    TextEdit::singleline(&mut window.options.capture_filename).desired_width(370.),
                );
                if r.changed() {
                    changed = true;
                }
                if ui.button("…").clicked() {
                    let files: Option<std::path::PathBuf> = rfd::FileDialog::new().save_file();
                    if let Some(path) = files {
                        if let Some(s) = path.to_str() {
                            window.options.capture_filename = s.to_string();
                            changed = true;
                        }
                    }
                }
            });
            ui.add_space(8.);
            ui.separator();
            ui.add_space(4.0);

            ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                if window.capture_session {
                    if ui
                        .button(fl!(crate::LANGUAGE_LOADER, "toolbar-stop-capture"))
                        .clicked()
                    {
                        window.capture_session = false;
                        close_dialog = true;
                    }
                } else if ui
                    .button(fl!(crate::LANGUAGE_LOADER, "capture-dialog-capture-button"))
                    .clicked()
                {
                    window.capture_session = true;
                    close_dialog = true;
                }

                if let Some(path) = Path::new(&window.options.capture_filename).parent() {
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
                    .button(fl!(crate::LANGUAGE_LOADER, "phonebook-cancel-button"))
                    .clicked()
                {
                    close_dialog = true;
                }
            });
        });

    if changed {
        window.show_capture_error = false;
        check_error!(window, window.options.store_options(), false);
    }

    if !open || close_dialog {
        window.mode = MainWindowMode::ShowTerminal;
    }
}
