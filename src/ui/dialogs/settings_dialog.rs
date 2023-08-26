use eframe::egui::{self, RichText};
use egui::{Layout, TextEdit, Vec2};
use i18n_embed_fl::fl;

use crate::{
    check_error,
    ui::{MainWindow, MainWindowMode},
    Scaling,
};
use lazy_static::lazy_static;
lazy_static! {
    static ref MONITOR_NAMES: [String; 6] = [
        fl!(crate::LANGUAGE_LOADER, "settings-monitor-color"),
        fl!(crate::LANGUAGE_LOADER, "settings-monitor-grayscale"),
        fl!(crate::LANGUAGE_LOADER, "settings-monitor-amber"),
        fl!(crate::LANGUAGE_LOADER, "settings-monitor-green"),
        fl!(crate::LANGUAGE_LOADER, "settings-monitor-apple2"),
        fl!(crate::LANGUAGE_LOADER, "settings-monitor-futuristic"),
    ];
}

#[derive(Default)]
pub struct DialogState {
    pub settings_category: usize,
}

pub fn show_settings(window: &mut MainWindow, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    let mut open = true;
    let mut close_dialog = false;
    let title = RichText::new(fl!(crate::LANGUAGE_LOADER, "settings-heading"));
    if ctx.input(|i| i.key_down(egui::Key::Escape)) {
        open = false;
    }

    let old_options = window.get_options().clone();
    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .fixed_size(Vec2::new(400., 300.))
        .resizable(false)
        .frame(egui::Frame::window(&ctx.style()))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
                let settings_category = window.settings_dialog.settings_category;
                if ui
                    .selectable_label(
                        settings_category == 0,
                        fl!(crate::LANGUAGE_LOADER, "settings-monitor-category"),
                    )
                    .clicked()
                {
                    window.settings_dialog.settings_category = 0;
                }
                if ui
                    .selectable_label(
                        settings_category == 1,
                        fl!(crate::LANGUAGE_LOADER, "settings-iemsi-category"),
                    )
                    .clicked()
                {
                    window.settings_dialog.settings_category = 1;
                }
                if ui
                    .selectable_label(
                        settings_category == 2,
                        fl!(crate::LANGUAGE_LOADER, "settings-terminal-category"),
                    )
                    .clicked()
                {
                    window.settings_dialog.settings_category = 2;
                }
                if ui
                    .selectable_label(
                        settings_category == 3,
                        fl!(crate::LANGUAGE_LOADER, "settings-keybinds-category"),
                    )
                    .clicked()
                {
                    window.settings_dialog.settings_category = 3;
                }
            });
            ui.separator();
            let settings_category = window.settings_dialog.settings_category;
            match settings_category {
                0 => show_monitor_settings(window, ui),
                1 => show_iemsi_settings(window, ui),
                2 => show_terminal_settings(window, ui),
                3 => crate::show_keybinds_settings(window, ui),
                _ => log::error!("Invalid settings category"),
            }
            ui.separator();
            ui.add_space(4.0);
            ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                if ui
                    .button(fl!(crate::LANGUAGE_LOADER, "dialing_directory-ok-button"))
                    .clicked()
                {
                    close_dialog = true;
                }
                if settings_category == 0
                    && ui
                        .button(fl!(crate::LANGUAGE_LOADER, "settings-reset-button"))
                        .clicked()
                {
                    window.state.options.reset_monitor_settings();
                }
                if settings_category == 3
                    && ui
                        .button(fl!(crate::LANGUAGE_LOADER, "settings-reset-button"))
                        .clicked()
                {
                    window.state.options.reset_keybindings();
                }
            });
        });

    if !open || close_dialog {
        if let MainWindowMode::ShowSettings(show_dialing_directory) = window.get_mode() {
            if show_dialing_directory {
                window.set_mode(MainWindowMode::ShowDialingDirectory);
            } else {
                window.set_mode(MainWindowMode::ShowTerminal);
            }
        }
    }

    if &old_options != window.get_options() {
        check_error!(window, window.get_options().store_options(), false);
    }
}

fn show_iemsi_settings(window: &mut MainWindow, ui: &mut egui::Ui) {
    ui.checkbox(
        &mut window.state.options.iemsi_autologin,
        fl!(crate::LANGUAGE_LOADER, "settings-iemsi-autologin-checkbox"),
    );

    egui::Grid::new("some_unique_id")
        .num_columns(2)
        .spacing([4.0, 8.0])
        .min_row_height(24.)
        .show(ui, |ui| {
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-alias"
                )));
            });
            ui.add(TextEdit::singleline(&mut window.state.options.iemsi_alias));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-location"
                )));
            });
            ui.add(TextEdit::singleline(
                &mut window.state.options.iemsi_location,
            ));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-data-phone"
                )));
            });
            ui.add(TextEdit::singleline(
                &mut window.state.options.iemsi_data_phone,
            ));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-voice-phone"
                )));
            });
            ui.add(TextEdit::singleline(
                &mut window.state.options.iemsi_voice_phone,
            ));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-birth-date"
                )));
            });
            ui.add(TextEdit::singleline(
                &mut window.state.options.iemsi_birth_date,
            ));
            ui.end_row();
        });
}

fn show_terminal_settings(window: &mut MainWindow, ui: &mut egui::Ui) {
    if ui
        .checkbox(
            &mut window.state.options.console_beep,
            fl!(
                crate::LANGUAGE_LOADER,
                "settings-terminal-console-beep-checkbox"
            ),
        )
        .changed()
    {
        check_error!(window, window.get_options().store_options(), false);
    }
    ui.add_space(16.0);
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "GitHub", "icy_term") {
        if ui
            .button(fl!(
                crate::LANGUAGE_LOADER,
                "settings-terminal-open-settings-dir-button"
            ))
            .clicked()
        {
            open::that(proj_dirs.config_dir()).unwrap();
        }
    }
    ui.add_space(8.0);
}

fn show_monitor_settings(window: &mut MainWindow, ui: &mut egui::Ui) {
    let text = match window.get_options().scaling {
        Scaling::Nearest => fl!(crate::LANGUAGE_LOADER, "settings-scaling-nearest"),
        Scaling::Linear => fl!(crate::LANGUAGE_LOADER, "settings-scaling-linear"),
        //   _ => "Error".to_string(),
    };
    egui::ComboBox::from_label(fl!(crate::LANGUAGE_LOADER, "settings-scaling"))
        .width(150.)
        .selected_text(RichText::new(text))
        .show_ui(ui, |ui| {
            for t in &Scaling::ALL {
                let label = RichText::new(format!("{t:?}"));
                let resp = ui.selectable_value(&mut window.state.options.scaling, *t, label);
                if resp.changed() {
                    check_error!(window, window.get_options().store_options(), false);
                }
            }
        });

    let cur_color = window.get_options().monitor_settings.monitor_type;
    egui::ComboBox::from_label(fl!(crate::LANGUAGE_LOADER, "settings-monitor-type"))
        .width(150.)
        .selected_text(&MONITOR_NAMES[cur_color])
        .show_ui(ui, |ui| {
            (0..MONITOR_NAMES.len()).for_each(|i| {
                let label = RichText::new(&MONITOR_NAMES[i]);
                let resp = ui.selectable_value(
                    &mut window.state.options.monitor_settings.monitor_type,
                    i,
                    label,
                );
                if resp.changed() {
                    check_error!(window, window.get_options().store_options(), false);
                }
            });
        });
    let old_settings = window.get_options().monitor_settings.clone();
    let use_filter = window.get_options().monitor_settings.use_filter;

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    ui.checkbox(
        &mut window.state.options.monitor_settings.use_filter,
        fl!(
            crate::LANGUAGE_LOADER,
            "settings-monitor-use-crt-filter-checkbox"
        ),
    );
    ui.add_enabled_ui(use_filter, |ui| {
        // todo: that should take full with, but doesn't work - egui bug ?
        ui.vertical_centered_justified(|ui| {
            ui.add(
                egui::Slider::new(
                    &mut window.state.options.monitor_settings.brightness,
                    0.0..=100.0,
                )
                .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-brightness")),
            );
            ui.add(
                egui::Slider::new(
                    &mut window.state.options.monitor_settings.contrast,
                    0.0..=100.0,
                )
                .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-contrast")),
            );
            ui.add(
                egui::Slider::new(
                    &mut window.state.options.monitor_settings.saturation,
                    0.0..=100.0,
                )
                .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-saturation")),
            );
            ui.add(
                egui::Slider::new(
                    &mut window.state.options.monitor_settings.gamma,
                    0.0..=100.0,
                )
                .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-gamma")),
            );
            /*  ui.add_enabled(
                use_filter,
                egui::Slider::new(
                    &mut window.buffer_view.lock().monitor_settings.light,
                    0.0..=100.0,
                )
                .text("Light"),
            );*/
            ui.add(
                egui::Slider::new(&mut window.state.options.monitor_settings.blur, 0.0..=100.0)
                    .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-blur")),
            );
            ui.add(
                egui::Slider::new(
                    &mut window.state.options.monitor_settings.curvature,
                    0.0..=100.0,
                )
                .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-curve")),
            );
            ui.add(
                egui::Slider::new(
                    &mut window.state.options.monitor_settings.scanlines,
                    0.0..=100.0,
                )
                .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-scanlines")),
            );
        });
    });

    ui.add_space(8.0);

    if old_settings != window.get_options().monitor_settings {
        check_error!(window, window.get_options().store_options(), false);
    }
}
