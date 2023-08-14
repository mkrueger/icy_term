use eframe::egui::{self, RichText};
use i18n_embed_fl::fl;

use super::{
    main_window_mod::{MainWindow, MainWindowMode},
    Scaling,
};
const MONITOR_NAMES: [&str; 6] = [
    "Color",
    "Grayscale",
    "Amber",
    "Green",
    "Apple ][",
    "Futuristic",
];

pub fn show_settings(window: &mut MainWindow, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut open = true;
    let mut close_dialog = false;
    let title = RichText::new(fl!(crate::LANGUAGE_LOADER, "settings-heading"));
    if ctx.input(|i| i.key_down(egui::Key::Escape)) {
        open = false;
    }

    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.add_space(8.0);
            egui::ComboBox::from_label(fl!(crate::LANGUAGE_LOADER, "settings-scaling"))
                .selected_text(RichText::new(format!("{:?}", window.options.scaling)))
                .show_ui(ui, |ui| {
                    for t in &Scaling::ALL {
                        let label = RichText::new(format!("{t:?}"));
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

            let cur_color = window.buffer_view.lock().monitor_settings.monitor_type;
            egui::ComboBox::from_label(fl!(crate::LANGUAGE_LOADER, "settings-monitor-type"))
                .selected_text(MONITOR_NAMES[cur_color])
                .show_ui(ui, |ui| {
                    for i in 0..MONITOR_NAMES.len() {
                        let t = MONITOR_NAMES[i];
                        let label = RichText::new(t);
                        let resp = ui.selectable_value(
                            &mut window.options.monitor_settings.monitor_type,
                            i,
                            label,
                        );
                        if resp.changed() {
                            window.handle_result(window.options.store_options(), false);
                            window.buffer_view.lock().monitor_settings.monitor_type = i;
                        }
                    }
                });
            let old_settings = window.buffer_view.lock().monitor_settings.clone();
            let use_filter = window.buffer_view.lock().monitor_settings.use_filter;

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.checkbox(
                &mut window.buffer_view.lock().monitor_settings.use_filter,
                "Use CRT filter",
            );

            ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.brightness,
                    0.0..=100.0,
                )
                .text("Brightness"),
            );
            ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.contrast,
                    0.0..=100.0,
                )
                .text("Contrast"),
            );
            ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.saturation,
                    0.0..=100.0,
                )
                .text("Saturation"),
            );
            ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.gamma,
                    0.0..=100.0,
                )
                .text("Gamma"),
            );
            /*  ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.light,
                    0.0..=100.0,
                )
                .text("Light"),
            );*/
            ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.blur,
                    0.0..=100.0,
                )
                .text("Blur"),
            );
            ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.curvature,
                    0.0..=100.0,
                )
                .text("Curve"),
            );
            ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.scanlines,
                    0.0..=100.0,
                )
                .text("Scanlines"),
            );
            ui.add_space(8.0);

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Reset").clicked() {
                    window.options.scaling = Scaling::Nearest;
                    window.buffer_view.lock().monitor_settings = super::MonitorSettings::default();
                }
                if ui
                    .button(fl!(crate::LANGUAGE_LOADER, "phonebook-ok-button"))
                    .clicked()
                {
                    close_dialog = true;
                }
            });

            let new_settings = window.buffer_view.lock().monitor_settings.clone();
            if old_settings != new_settings {
                window.options.monitor_settings = new_settings;
                window.handle_result(window.options.store_options(), false);
            }
        });

    if !open || close_dialog {
        if let MainWindowMode::ShowSettings(show_phonebook) = window.mode {
            if show_phonebook {
                window.mode = MainWindowMode::ShowPhonebook;
            } else {
                window.mode = MainWindowMode::ShowTerminal;
            }
        }
    }
}
