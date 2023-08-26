use eframe::egui::{self, RichText};
use egui::{Layout, TextEdit, Vec2};
use i18n_embed_fl::fl;

use crate::{
    ui::{MainWindowMode, MainWindowState},
    KeyBindings, Scaling,
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

#[derive(Clone, Debug)]
pub(crate) enum Command {
    SwitchSettingsCategory(usize),
    CloseDialog,
    OpenSettingsFolder,
    ResetMonitorSettings,
    ResetKeybindSettings,
    SetIEMSI(crate::IEMSISettings),
    SetMonitorSettings(icy_engine_egui::MonitorSettings),
    ChangeOpenglScaling(Scaling),
    UpdateKeybinds(KeyBindings),
    ChangeConsoleBeep(bool),
}

type ShowSettingsCallback = fn(&MainWindowState, ui: &mut egui::Ui) -> Option<Command>;
type ResetCommand = Option<Command>;

lazy_static! {
    static ref SETTING_CATEGORIES: [(String, ShowSettingsCallback, ResetCommand); 4] = [
        (
            fl!(crate::LANGUAGE_LOADER, "settings-monitor-category"),
            show_monitor_settings,
            Some(Command::ResetMonitorSettings)
        ),
        (
            fl!(crate::LANGUAGE_LOADER, "settings-iemsi-category"),
            show_iemsi_settings,
            None
        ),
        (
            fl!(crate::LANGUAGE_LOADER, "settings-terminal-category"),
            show_terminal_settings,
            None
        ),
        (
            fl!(crate::LANGUAGE_LOADER, "settings-keybinds-category"),
            crate::show_keybinds_settings,
            Some(Command::ResetKeybindSettings)
        ),
    ];
}

impl MainWindowState {
    pub fn show_settings(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut result = None;

        let mut open = true;
        let title = RichText::new(fl!(crate::LANGUAGE_LOADER, "settings-heading"));
        if ctx.input(|i| i.key_down(egui::Key::Escape)) {
            open = false;
        }

        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .fixed_size(Vec2::new(400., 300.))
            .resizable(false)
            .frame(egui::Frame::window(&ctx.style()))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::widgets::global_dark_light_mode_switch(ui);
                    let settings_category = self.settings_dialog.settings_category;
                    for i in 0..SETTING_CATEGORIES.len() {
                        if ui
                            .selectable_label(settings_category == i, &SETTING_CATEGORIES[i].0)
                            .clicked()
                        {
                            result = Some(Command::SwitchSettingsCategory(i));
                        }
                    }
                });
                ui.separator();
                let settings_category = self.settings_dialog.settings_category;
                if let Some(cat) = SETTING_CATEGORIES.get(settings_category) {
                    if let Some(cmd) = cat.1(self, ui) {
                        result = Some(cmd);
                    }
                } else {
                    ui.colored_label(
                        ui.style().visuals.error_fg_color,
                        "Invalid settings category",
                    );
                }

                ui.separator();
                ui.add_space(4.0);
                ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .button(fl!(crate::LANGUAGE_LOADER, "dialing_directory-ok-button"))
                        .clicked()
                    {
                        result = Some(Command::CloseDialog);
                    }

                    let settings_category = self.settings_dialog.settings_category;
                    if let Some(cat) = SETTING_CATEGORIES.get(settings_category) {
                        if let Some(reset_cmd) = &cat.2 {
                            if ui
                                .button(fl!(crate::LANGUAGE_LOADER, "settings-reset-button"))
                                .clicked()
                            {
                                result = Some(reset_cmd.clone());
                            }
                        }
                    }
                });
            });

        if !open {
            result = Some(Command::CloseDialog);
        }

        handle_command(self, result);
    }
}

fn show_iemsi_settings(state: &MainWindowState, ui: &mut egui::Ui) -> Option<Command> {
    let mut iemsi = state.options.iemsi.clone();
    ui.checkbox(
        &mut iemsi.autologin,
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
            ui.add(TextEdit::singleline(&mut iemsi.alias));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-location"
                )));
            });
            ui.add(TextEdit::singleline(&mut iemsi.location));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-data-phone"
                )));
            });
            ui.add(TextEdit::singleline(&mut iemsi.data_phone));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-voice-phone"
                )));
            });
            ui.add(TextEdit::singleline(&mut iemsi.voice_phone));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "settings-iemsi-birth-date"
                )));
            });
            ui.add(TextEdit::singleline(&mut iemsi.birth_date));
            ui.end_row();
        });

    if iemsi == state.options.iemsi {
        None
    } else {
        Some(Command::SetIEMSI(iemsi))
    }
}

fn show_terminal_settings(state: &MainWindowState, ui: &mut egui::Ui) -> Option<Command> {
    let mut result = None;
    let mut beep = state.options.console_beep;

    if ui
        .checkbox(
            &mut beep,
            fl!(
                crate::LANGUAGE_LOADER,
                "settings-terminal-console-beep-checkbox"
            ),
        )
        .changed()
    {
        result = Some(Command::ChangeConsoleBeep(beep));
    }

    ui.add_space(16.0);
    if ui
        .button(fl!(
            crate::LANGUAGE_LOADER,
            "settings-terminal-open-settings-dir-button"
        ))
        .clicked()
    {
        result = Some(Command::OpenSettingsFolder);
    }
    ui.add_space(8.0);

    result
}

fn show_monitor_settings(state: &MainWindowState, ui: &mut egui::Ui) -> Option<Command> {
    let mut result = None;

    let text = match state.options.scaling {
        Scaling::Nearest => fl!(crate::LANGUAGE_LOADER, "settings-scaling-nearest"),
        Scaling::Linear => fl!(crate::LANGUAGE_LOADER, "settings-scaling-linear"),
        //   _ => "Error".to_string(),
    };
    egui::ComboBox::from_label(fl!(crate::LANGUAGE_LOADER, "settings-scaling"))
        .width(150.)
        .selected_text(RichText::new(text))
        .show_ui(ui, |ui| {
            let mut scaling = state.options.scaling;
            for t in &Scaling::ALL {
                let label = RichText::new(format!("{t:?}"));
                if ui.selectable_value(&mut scaling, *t, label).changed() {
                    result = Some(Command::ChangeOpenglScaling(scaling));
                }
            }
        });

    let mut monitor_settings = state.options.monitor_settings.clone();

    let cur_color = monitor_settings.monitor_type;
    egui::ComboBox::from_label(fl!(crate::LANGUAGE_LOADER, "settings-monitor-type"))
        .width(150.)
        .selected_text(&MONITOR_NAMES[cur_color])
        .show_ui(ui, |ui| {
            (0..MONITOR_NAMES.len()).for_each(|i| {
                let label = RichText::new(&MONITOR_NAMES[i]);
                ui.selectable_value(&mut monitor_settings.monitor_type, i, label);
            });
        });
    let use_filter = monitor_settings.use_filter;

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);

    ui.checkbox(
        &mut monitor_settings.use_filter,
        fl!(
            crate::LANGUAGE_LOADER,
            "settings-monitor-use-crt-filter-checkbox"
        ),
    );
    ui.add_enabled_ui(use_filter, |ui| {
        // todo: that should take full with, but doesn't work - egui bug ?
        ui.vertical_centered_justified(|ui| {
            ui.add(
                egui::Slider::new(&mut monitor_settings.brightness, 0.0..=100.0)
                    .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-brightness")),
            );
            ui.add(
                egui::Slider::new(&mut monitor_settings.contrast, 0.0..=100.0)
                    .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-contrast")),
            );
            ui.add(
                egui::Slider::new(&mut monitor_settings.saturation, 0.0..=100.0)
                    .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-saturation")),
            );
            ui.add(
                egui::Slider::new(&mut monitor_settings.gamma, 0.0..=100.0)
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
                egui::Slider::new(&mut monitor_settings.blur, 0.0..=100.0)
                    .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-blur")),
            );
            ui.add(
                egui::Slider::new(&mut monitor_settings.curvature, 0.0..=100.0)
                    .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-curve")),
            );
            ui.add(
                egui::Slider::new(&mut monitor_settings.scanlines, 0.0..=100.0)
                    .text(fl!(crate::LANGUAGE_LOADER, "settings-monitor-scanlines")),
            );
        });
    });

    ui.add_space(8.0);
    if monitor_settings != state.options.monitor_settings {
        result = Some(Command::SetMonitorSettings(monitor_settings));
    }
    result
}

fn handle_command(state: &mut MainWindowState, command_opt: Option<Command>) {
    match command_opt {
        Some(Command::CloseDialog) => {
            state.mode = MainWindowMode::ShowTerminal;
        }
        Some(Command::SwitchSettingsCategory(category)) => {
            state.settings_dialog.settings_category = category;
        }
        Some(Command::ResetMonitorSettings) => {
            state.options.reset_monitor_settings();
            state.store_options();
        }
        Some(Command::ResetKeybindSettings) => {
            state.options.reset_keybindings();
            state.store_options();
        }
        Some(Command::OpenSettingsFolder) => {
            if let Some(proj_dirs) = directories::ProjectDirs::from("com", "GitHub", "icy_term") {
                open::that(proj_dirs.config_dir()).unwrap();
            }
        }
        Some(Command::SetIEMSI(iemsi)) => {
            state.options.iemsi = iemsi;
            state.store_options();
        }
        Some(Command::SetMonitorSettings(monitor_settings)) => {
            state.options.monitor_settings = monitor_settings;
            state.store_options();
        }
        Some(Command::ChangeOpenglScaling(scaling)) => {
            state.options.scaling = scaling;
            state.store_options();
        }
        Some(Command::UpdateKeybinds(keybinds)) => {
            state.options.bind = keybinds;
            state.store_options();
        }
        Some(Command::ChangeConsoleBeep(beep)) => {
            state.options.console_beep = beep;
            state.store_options();
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use icy_engine_egui::MonitorSettings;

    use crate::{
        ui::{
            dialogs::settings_dialog::{handle_command, SETTING_CATEGORIES},
            MainWindowState,
        },
        IEMSISettings, KeyBindings, Options, Scaling,
    };

    #[test]
    fn test_close_dialog() {
        let mut state: MainWindowState = MainWindowState::default();
        state.mode = super::MainWindowMode::ShowSettings;
        handle_command(&mut state, Some(super::Command::CloseDialog));
        assert!(matches!(state.mode, super::MainWindowMode::ShowTerminal));
        assert!(!state.options_written);
    }

    #[test]
    fn test_switch_category_dialog() {
        let mut state: MainWindowState = MainWindowState::default();
        for i in 0..SETTING_CATEGORIES.len() {
            handle_command(&mut state, Some(super::Command::SwitchSettingsCategory(i)));
            assert_eq!(i, state.settings_dialog.settings_category);
        }
        assert!(!state.options_written);
    }

    #[test]
    fn test_reset_monitor_settings() {
        let mut state: MainWindowState = MainWindowState::default();
        let mut opt: Options = Options::default();
        opt.scaling = Scaling::Linear;
        opt.monitor_settings.blur = 0.0;
        opt.monitor_settings.brightness = 1.0;
        state.options = opt;

        assert_ne!(Options::default().scaling, state.options.scaling);
        assert_ne!(
            Options::default().monitor_settings,
            state.options.monitor_settings
        );
        handle_command(&mut state, Some(super::Command::ResetMonitorSettings));
        assert_eq!(Options::default().scaling, state.options.scaling);
        assert_eq!(
            Options::default().monitor_settings,
            state.options.monitor_settings
        );
        assert!(state.options_written);
    }

    #[test]
    fn test_reset_keybindings() {
        let mut state: MainWindowState = MainWindowState::default();
        let mut opt = Options::default();
        opt.bind.download = None;
        opt.bind.full_screen = None;
        state.options = opt;

        assert_ne!(Options::default().bind, state.options.bind);
        handle_command(&mut state, Some(super::Command::ResetKeybindSettings));
        assert_eq!(Options::default().bind, state.options.bind);
        assert!(state.options_written);
    }

    #[test]
    fn test_change_console_beep() {
        let mut state: MainWindowState = MainWindowState::default();
        handle_command(&mut state, Some(super::Command::ChangeConsoleBeep(false)));
        assert_ne!(Options::default().console_beep, state.options.console_beep);
        assert!(state.options_written);
    }

    #[test]
    fn test_set_keybindings() {
        let mut state: MainWindowState = MainWindowState::default();
        let mut bind = KeyBindings::default();
        bind.download = None;
        bind.full_screen = None;
        handle_command(&mut state, Some(super::Command::UpdateKeybinds(bind)));
        assert_ne!(KeyBindings::default(), state.options.bind);
        assert!(state.options_written);
    }

    #[test]
    fn test_set_monitor_settings() {
        let mut state: MainWindowState = MainWindowState::default();
        let mut settings = MonitorSettings::default();
        settings.blur = 0.0;
        settings.brightness = 1.0;
        handle_command(
            &mut state,
            Some(super::Command::SetMonitorSettings(settings)),
        );
        assert_ne!(MonitorSettings::default(), state.options.monitor_settings);
        assert!(state.options_written);
    }

    #[test]
    fn test_set_iemsi_settings() {
        let mut state: MainWindowState = MainWindowState::default();
        let mut settings = IEMSISettings::default();
        settings.alias = "foo".to_string();
        settings.voice_phone = "42".to_string();
        handle_command(&mut state, Some(super::Command::SetIEMSI(settings)));
        assert_ne!(IEMSISettings::default(), state.options.iemsi);
        assert!(state.options_written);
    }

    #[test]
    fn test_change_scaling() {
        let mut state: MainWindowState = MainWindowState::default();
        handle_command(
            &mut state,
            Some(super::Command::ChangeOpenglScaling(Scaling::Linear)),
        );
        assert_eq!(Scaling::Linear, state.options.scaling);
        assert!(state.options_written);
    }
}
