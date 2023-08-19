use eframe::egui::{self, RichText};
use egui::TextEdit;
use i18n_embed_fl::fl;

use crate::ui::{MainWindow, MainWindowMode};

#[cfg(not(target_arch = "wasm32"))]
pub fn show_dialog(window: &mut MainWindow, ctx: &egui::Context) {
    use std::path::Path;

    use egui::Vec2;

    let mut open = true;
    let mut close_dialog = false;
    let mut changed = true;
    if ctx.input(|i| i.key_down(egui::Key::Escape)) {
        open = false;
    }

    egui::Window::new("Capture")
        .open(&mut open)
        .collapsible(true)
        .default_size(Vec2::new(400., 300.))
        .resizable(true)
        .show(ctx, |ui| {
            ui.checkbox(
                &mut window.capture_session,
                fl!(crate::LANGUAGE_LOADER, "capture-dialog-capture-checkbox"),
            );

            ui.horizontal(|ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "capture-dialog-capture-label"
                )));
                let r = ui.add(TextEdit::singleline(&mut window.options.capture_filename));
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
                if let Some(path) = Path::new(&window.options.capture_filename).parent() {
                    if ui.button("Open folder…").clicked() {
                        if let Some(s) = path.to_str() {
                            if let Err(err) = open::that(s) {
                                log::error!("Failed to open folder: {}", err);
                            }
                        }
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button(fl!(crate::LANGUAGE_LOADER, "phonebook-ok-button"))
                    .clicked()
                {
                    close_dialog = true;
                }
            });
        });

    if changed {
        window.show_capture_error = false;
        window.handle_result(window.options.store_options(), false);
    }

    if !open || close_dialog {
        window.mode = MainWindowMode::ShowTerminal;
    }
}
