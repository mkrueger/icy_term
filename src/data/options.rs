use std::{
    fs::{self, File},
    io::Write,
    time::Duration,
};

use egui::Modifiers;
use egui_bind::KeyOrPointer;
use i18n_embed_fl::fl;
use icy_engine_egui::MonitorSettings;
use toml::Value;

use crate::TerminalResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scaling {
    #[default]
    Nearest,
    Linear,
}

impl Scaling {
    pub const ALL: [Scaling; 2] = [Scaling::Nearest, Scaling::Linear];

    #[must_use]
    pub fn get_filter(&self) -> i32 {
        match self {
            Scaling::Nearest => glow::NEAREST as i32,
            Scaling::Linear => glow::LINEAR as i32,
        }
    }
}

pub struct Key {
    pub description: String,
    pub key: egui::Key,
    pub modifiers: Modifiers,
}

type KeyType = Option<(KeyOrPointer, Modifiers)>;

macro_rules! keys {
    ($( ($l:ident, $key:ident, $mod: ident, $translation: expr) ),* ) => {
        #[derive(Debug, Clone, PartialEq)]
        pub struct KeyBindings {
            $(
                pub $l: KeyType,
            )*
        }

        impl Default for KeyBindings {
            fn default() -> Self {
                Self {
                    $(
                        $l: Some((KeyOrPointer::Key(egui::Key::$key), Modifiers::$mod)),
                    )*
                }
            }
        }

        fn parse_keybinding_settings(options: &mut Options, key_settings: &toml::map::Map<String, Value>) {
            for (k, v) in key_settings {
                match k.as_str() {
                    $(
                        stringify!($l) =>  {
                            if let Value::String(str) = v {
                                options.bind.$l = parse_key_binding(str);
                            }
                        }
                    )*
                    _ => {}
                }
            }
        }

        fn write_keybindings(file: &mut File, bind: &KeyBindings) -> TerminalResult<()> {
            file.write_all("[KEYBINDINGS]\n".to_string().as_bytes())?;
            $(
                file.write_all(
                    format!(
                        "{} = \"{}\"\n",
                        stringify!($l),
                        convert_to_string(bind.$l)
                    )
                    .as_bytes(),
                )?;
            )*
            Ok(())
        }

        pub fn show_keybinds_settings(window: &mut crate::ui::MainWindow, ui: &mut egui::Ui) {
            egui::Grid::new("keybinds_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    $(
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(fl!(crate::LANGUAGE_LOADER, $translation));
                        });
                        ui.add(egui_bind::Bind::new(
                            stringify!($l),
                            &mut window.options.bind.$l,
                        ));
                        ui.end_row();
                    )*
                });
        }

    }
}

fn parse_key_binding(str: impl Into<String>) -> KeyType {
    let str: String = str.into();
    let parts = str.split('|').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 2 {
        return None;
    }
    let key = parse_key(parts[0]);
    if parts[1] == "Alt" {
        Some((KeyOrPointer::Key(key), Modifiers::ALT))
    } else if parts[1] == "None" {
        Some((KeyOrPointer::Key(key), Modifiers::NONE))
    } else {
        None
    }
}

// Generated from "egui::Key::name()"
// \s*(.*)\s*=>\s*(.*)\s*,
// replace:
// $2 => egui::$1,
fn parse_key(s: &str) -> egui::Key {
    match s {
        "Down" => egui::Key::ArrowDown,
        "Left" => egui::Key::ArrowLeft,
        "Right" => egui::Key::ArrowRight,
        "Up" => egui::Key::ArrowUp,
        "Escape" => egui::Key::Escape,
        "Tab" => egui::Key::Tab,
        "Backspace" => egui::Key::Backspace,
        "Enter" => egui::Key::Enter,
        "Space" => egui::Key::Space,
        "Insert" => egui::Key::Insert,
        "Delete" => egui::Key::Delete,
        "Home" => egui::Key::Home,
        "End" => egui::Key::End,
        "PageUp" => egui::Key::PageUp,
        "PageDown" => egui::Key::PageDown,
        "Minus" => egui::Key::Minus,
        "Plus" => egui::Key::PlusEquals,
        "0" => egui::Key::Num0,
        "1" => egui::Key::Num1,
        "2" => egui::Key::Num2,
        "3" => egui::Key::Num3,
        "4" => egui::Key::Num4,
        "5" => egui::Key::Num5,
        "6" => egui::Key::Num6,
        "7" => egui::Key::Num7,
        "8" => egui::Key::Num8,
        "9" => egui::Key::Num9,
        "A" => egui::Key::A,
        "B" => egui::Key::B,
        "C" => egui::Key::C,
        "D" => egui::Key::D,
        "E" => egui::Key::E,
        "F" => egui::Key::F,
        "G" => egui::Key::G,
        "H" => egui::Key::H,
        "I" => egui::Key::I,
        "J" => egui::Key::J,
        "K" => egui::Key::K,
        "L" => egui::Key::L,
        "M" => egui::Key::M,
        "N" => egui::Key::N,
        "O" => egui::Key::O,
        "P" => egui::Key::P,
        "Q" => egui::Key::Q,
        "R" => egui::Key::R,
        "S" => egui::Key::S,
        "T" => egui::Key::T,
        "U" => egui::Key::U,
        "V" => egui::Key::V,
        "W" => egui::Key::W,
        "X" => egui::Key::X,
        "Y" => egui::Key::Y,
        "Z" => egui::Key::Z,
        "F1" => egui::Key::F1,
        "F2" => egui::Key::F2,
        "F3" => egui::Key::F3,
        "F4" => egui::Key::F4,
        "F5" => egui::Key::F5,
        "F6" => egui::Key::F6,
        "F7" => egui::Key::F7,
        "F8" => egui::Key::F8,
        "F9" => egui::Key::F9,
        "F10" => egui::Key::F10,
        "F11" => egui::Key::F11,
        "F12" => egui::Key::F12,
        "F13" => egui::Key::F13,
        "F14" => egui::Key::F14,
        "F15" => egui::Key::F15,
        "F16" => egui::Key::F16,
        "F17" => egui::Key::F17,
        "F18" => egui::Key::F18,
        "F19" => egui::Key::F19,
        _ => egui::Key::F20,
    }
}

fn convert_to_string(key: KeyType) -> String {
    match key {
        Some((key, modifier)) => {
            if let KeyOrPointer::Key(key) = key {
                let m = if modifier.alt { "Alt" } else { "None" };
                format!("{}|{}", key.name(), m)
            } else {
                "None".to_string()
            }
        }
        None => "None".to_string(),
    }
}

keys![
    (hangup, H, ALT, "settings-keybinds-disconnect"),
    (
        dialing_directory,
        D,
        ALT,
        "settings-keybinds-dialing-directory"
    ),
    (send_login_pw, L, ALT, "settings-keybinds-send-login"),
    (upload, PageUp, ALT, "settings-keybinds-upload"),
    (download, PageDown, ALT, "settings-keybinds-download"),
    (clear_screen, C, ALT, "settings-keybinds-clear-screen"),
    (quit, X, ALT, "settings-keybinds-quit"),
    (
        full_screen,
        F11,
        NONE,
        "settings-keybinds-toggle-fullscreen"
    ),
    (show_settings, O, ALT, "settings-keybinds-show-settings"),
    (show_capture, P, ALT, "settings-keybinds-capture-control")
];

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub scaling: Scaling,
    pub connect_timeout: Duration,
    pub monitor_settings: MonitorSettings,
    pub capture_filename: String,

    pub console_beep: bool,

    pub iemsi_autologin: bool,
    pub iemsi_alias: String,
    pub iemsi_location: String,
    pub iemsi_data_phone: String,
    pub iemsi_voice_phone: String,
    pub bind: KeyBindings,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scaling: Scaling::default(),
            connect_timeout: Duration::default(),
            monitor_settings: MonitorSettings::default(),
            capture_filename: String::default(),
            iemsi_autologin: true,
            iemsi_alias: String::default(),
            iemsi_location: String::default(),
            iemsi_data_phone: String::default(),
            iemsi_voice_phone: String::default(),
            console_beep: true,
            bind: KeyBindings::default(),
        }
    }
}

impl Options {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn load_options() -> TerminalResult<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "GitHub", "icy_term") {
            let options_file = proj_dirs.config_dir().join("options.toml");
            if options_file.exists() {
                match fs::read_to_string(&options_file) {
                    Ok(content) => {
                        return Ok(Options::from_str(&content));
                    }
                    Err(err) => {
                        return Err(err.into());
                    }
                }
            }
        }
        Ok(Options::default())
    }

    /// Returns the store options of this [`Options`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn store_options(&self) -> TerminalResult<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "GitHub", "icy_term") {
            let options_file = proj_dirs.config_dir().join("options.toml");

            let mut file = File::create(options_file)?;
            file.write_all(b"version = \"1.1\"\n")?;

            file.write_all(format!("scaling = \"{:?}\"\n", self.scaling).as_bytes())?;
            file.write_all(
                format!("use_crt_filter = {:?}\n", self.monitor_settings.use_filter).as_bytes(),
            )?;
            file.write_all(
                format!("monitor_type = {:?}\n", self.monitor_settings.monitor_type).as_bytes(),
            )?;
            file.write_all(
                format!("monitor_gamma = {:?}\n", self.monitor_settings.gamma).as_bytes(),
            )?;
            file.write_all(
                format!("monitor_contrast = {:?}\n", self.monitor_settings.contrast).as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_saturation = {:?}\n",
                    self.monitor_settings.saturation
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_brightness = {:?}\n",
                    self.monitor_settings.brightness
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!("monitor_blur = {:?}\n", self.monitor_settings.blur).as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_curvature = {:?}\n",
                    self.monitor_settings.curvature
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_scanlines = {:?}\n",
                    self.monitor_settings.scanlines
                )
                .as_bytes(),
            )?;

            file.write_all(format!("console_beep = {}\n", self.console_beep).as_bytes())?;

            file.write_all("[IEMSI]\n".to_string().as_bytes())?;

            if !self.iemsi_autologin {
                file.write_all(format!("autologin = {}\n", self.iemsi_autologin).as_bytes())?;
            }
            if !self.iemsi_location.is_empty() {
                file.write_all(format!("location = \"{}\"\n", self.iemsi_location).as_bytes())?;
            }
            if !self.iemsi_alias.is_empty() {
                file.write_all(format!("alias = \"{}\"\n", self.iemsi_alias).as_bytes())?;
            }
            if !self.iemsi_data_phone.is_empty() {
                file.write_all(format!("data_phone = \"{}\"\n", self.iemsi_data_phone).as_bytes())?;
            }
            if !self.iemsi_voice_phone.is_empty() {
                file.write_all(
                    format!("voice_phone = \"{}\"\n", self.iemsi_voice_phone).as_bytes(),
                )?;
            }

            if !self.iemsi_autologin {
                file.write_all(format!("autologin = {}\n", self.iemsi_autologin).as_bytes())?;
            }

            write_keybindings(&mut file, &self.bind)?;

            file.flush()?;
        }
        Ok(())
    }

    fn from_str(input_text: &str) -> Options {
        let value = input_text.parse::<Value>().unwrap();
        let mut result = Options::default();
        parse_value(&mut result, &value);
        result
    }
}

fn parse_value(options: &mut Options, value: &Value) {
    match value {
        Value::Array(array) => {
            for v in array {
                parse_value(options, v);
            }
        }
        Value::Table(table) => {
            for (k, v) in table {
                match k.as_str() {
                    "scaling" => {
                        if let Value::String(str) = v {
                            match str.as_str() {
                                "Nearest" => options.scaling = Scaling::Nearest,
                                "Linear" => options.scaling = Scaling::Linear,
                                _ => {}
                            }
                        }
                    }
                    "use_crt_filter" => {
                        if let Value::Boolean(b) = v {
                            options.monitor_settings.use_filter = *b;
                        }
                    }
                    "monitor_type" => {
                        if let Value::Integer(b) = v {
                            options.monitor_settings.monitor_type = *b as usize;
                        }
                    }
                    "monitor_gamma" => {
                        if let Value::Float(f) = v {
                            options.monitor_settings.gamma = *f as f32;
                        }
                    }
                    "monitor_contrast" => {
                        if let Value::Float(f) = v {
                            options.monitor_settings.contrast = *f as f32;
                        }
                    }
                    "monitor_saturation" => {
                        if let Value::Float(f) = v {
                            options.monitor_settings.saturation = *f as f32;
                        }
                    }
                    "monitor_brightness" => {
                        if let Value::Float(f) = v {
                            options.monitor_settings.brightness = *f as f32;
                        }
                    }
                    "monitor_blur" => {
                        if let Value::Float(f) = v {
                            options.monitor_settings.blur = *f as f32;
                        }
                    }
                    "monitor_curvature" => {
                        if let Value::Float(f) = v {
                            options.monitor_settings.curvature = *f as f32;
                        }
                    }
                    "monitor_scanlines" => {
                        if let Value::Float(f) = v {
                            options.monitor_settings.scanlines = *f as f32;
                        }
                    }
                    "IEMSI" => {
                        if let Value::Table(iemsi_settings) = v {
                            parse_iemsi_settings(options, iemsi_settings);
                        }
                    }
                    "KEYBINDINGS" => {
                        if let Value::Table(keybind_settings) = v {
                            parse_keybinding_settings(options, keybind_settings);
                        }
                    }

                    "console_beep" => {
                        if let Value::Boolean(b) = v {
                            options.console_beep = *b;
                        }
                    }

                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn parse_iemsi_settings(options: &mut Options, iemsi_settings: &toml::map::Map<String, Value>) {
    for (k, v) in iemsi_settings {
        match k.as_str() {
            "autologin" => {
                if let Value::Boolean(autologin) = v {
                    options.iemsi_autologin = *autologin;
                }
            }
            "location" => {
                if let Value::String(str) = v {
                    options.iemsi_location = str.clone();
                }
            }
            "alias" => {
                if let Value::String(str) = v {
                    options.iemsi_alias = str.clone();
                }
            }
            "data_phone" => {
                if let Value::String(str) = v {
                    options.iemsi_data_phone = str.clone();
                }
            }
            "voice_phone" => {
                if let Value::String(str) = v {
                    options.iemsi_voice_phone = str.clone();
                }
            }
            _ => {}
        }
    }
}
