use eframe::{
    egui::{self, RichText},
};
use i18n_embed_fl::fl;

use super::main_window::{MainWindow, MainWindowMode, PostProcessing, Scaling};

pub fn show_settings(window: &mut MainWindow, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut open = true;
    let title = RichText::new(fl!(crate::LANGUAGE_LOADER, "settings-heading"));

    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(fl!(crate::LANGUAGE_LOADER, "settings-scaling")));
                egui::ComboBox::from_id_source("settings_combobox_1")
                    .selected_text(
                        RichText::new(format!("{:?}", window.options.scaling))
                            ,
                    )
                    .show_ui(ui, |ui| {
                        for t in &Scaling::ALL {
                            let label = RichText::new(format!("{:?}", t));
                            let resp = ui.selectable_value(&mut window.options.scaling, *t, label);
                            if resp.changed() {
                                window.handle_result(window.options.store_options(), false);
                                window
                                    .buffer_view
                                    .lock()
                                    .set_scaling(window.options.scaling);
                            }
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new(fl!(crate::LANGUAGE_LOADER, "settings-post-processing")));
                egui::ComboBox::from_id_source("settings_combobox_2")
                    .selected_text(
                        RichText::new(format!("{:?}", window.options.post_processing))
                            ,
                    )
                    .show_ui(ui, |ui| {
                        for t in &PostProcessing::ALL {
                            let label = RichText::new(format!("{:?}", t));
                            let resp =
                                ui.selectable_value(&mut window.options.post_processing, *t, label);
                            if resp.changed() {
                                window.handle_result(window.options.store_options(), false);
                                window
                                    .buffer_view
                                    .lock()
                                    .set_post_processing(window.options.post_processing);
                            }
                        }
                    });
            });
        });

    if !open {
        match window.mode {
            MainWindowMode::ShowSettings(show_phonebook) => {
                if show_phonebook {
                    window.mode = MainWindowMode::ShowPhonebook;
                } else {
                    window.mode = MainWindowMode::ShowTerminal;
                }
            },
            _ => {}
        }
    }
}
