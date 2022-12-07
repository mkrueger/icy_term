use eframe::{egui::{self, RichText}, epaint::FontId};

use super::main_window::{MainWindow, MainWindowMode, Scaling, PostProcessing};


pub fn show_settings(window: &mut MainWindow, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut open = true;
    let text_style = FontId::proportional(26.);
    let title = RichText::new("Settings").font(text_style);

    egui::Window::new(title)
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let text_style = FontId::proportional(22.);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Scaling").font(text_style.clone()));
            egui::ComboBox::from_id_source("settings_combobox_1")
            .selected_text(RichText::new(format!("{:?}", window.options.scaling)).font(text_style.clone()))
            .show_ui(ui, |ui| {
                for t in &Scaling::ALL {
                    let label = RichText::new(format!("{:?}", t)).font(text_style.clone());
                    let resp = ui.selectable_value(&mut window.options.scaling, *t, label);
                    if resp.changed() {
                        window.handle_result(window.options.store_options(), false);
                        window.buffer_view.lock().set_scaling(window.options.scaling);
                    }
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Post processing").font(text_style.clone()));
            egui::ComboBox::from_id_source("settings_combobox_2")
            .selected_text(RichText::new(format!("{:?}", window.options.post_processing)).font(text_style.clone()))
            .show_ui(ui, |ui| {
                for t in &PostProcessing::ALL {
                    let label = RichText::new(format!("{:?}", t)).font(text_style.clone());
                    let resp = ui.selectable_value(&mut window.options.post_processing, *t, label);
                    if resp.changed() {
                        window.handle_result(window.options.store_options(), false);
                        window.buffer_view.lock().set_post_processing(window.options.post_processing);
                    }
                }
            });
        });

    });

    if !open {
        window.mode = MainWindowMode::ShowPhonebook;
    }
}
