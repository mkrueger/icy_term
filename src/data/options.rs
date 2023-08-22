use std::{
    fs::{self, File},
    io::Write,
    time::Duration,
};

use egui::Modifiers;
use egui_bind::KeyOrPointer;
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
}

pub const MONO_COLORS: [(u8, u8, u8); 5] = [
    (0xFF, 0xFF, 0xFF), // Black / White
    (0xFF, 0x81, 0x00), // Amber
    (0x0C, 0xCC, 0x68), // Green
    (0x00, 0xD5, 0x6D), // Apple ][
    (0x72, 0x9F, 0xCF), // Futuristic
];

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorSettings {
    pub use_filter: bool,

    pub monitor_type: usize,

    pub gamma: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub light: f32,
    pub blur: f32,
    pub curvature: f32,
    pub scanlines: f32,
}

impl Default for MonitorSettings {
    fn default() -> Self {
        Self {
            use_filter: false,
            monitor_type: 0,
            gamma: 50.,
            contrast: 50.,
            saturation: 50.,
            brightness: 30.,
            light: 40.,
            blur: 30.,
            curvature: 10.,
            scanlines: 10.,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyBindings {
    pub clear_screen: Option<(KeyOrPointer, Modifiers)>,
    pub dialing_directory: Option<(KeyOrPointer, Modifiers)>,
    pub hangup: Option<(KeyOrPointer, Modifiers)>,
    pub send_login_pw: Option<(KeyOrPointer, Modifiers)>,
    pub show_settings: Option<(KeyOrPointer, Modifiers)>,
    pub show_capture: Option<(KeyOrPointer, Modifiers)>,
    pub quit: Option<(KeyOrPointer, Modifiers)>,
    pub full_screen: Option<(KeyOrPointer, Modifiers)>,
    pub upload: Option<(KeyOrPointer, Modifiers)>,
    pub download: Option<(KeyOrPointer, Modifiers)>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            clear_screen: Some((KeyOrPointer::Key(egui::Key::C), Modifiers::ALT)),
            dialing_directory: Some((KeyOrPointer::Key(egui::Key::D), Modifiers::ALT)),
            hangup: Some((KeyOrPointer::Key(egui::Key::H), Modifiers::ALT)),
            send_login_pw: Some((KeyOrPointer::Key(egui::Key::L), Modifiers::ALT)),
            show_settings: Some((KeyOrPointer::Key(egui::Key::O), Modifiers::ALT)),
            show_capture: Some((KeyOrPointer::Key(egui::Key::P), Modifiers::ALT)),
            quit: Some((KeyOrPointer::Key(egui::Key::X), Modifiers::ALT)),
            full_screen: Some((KeyOrPointer::Key(egui::Key::F11), Modifiers::NONE)),
            upload: Some((KeyOrPointer::Key(egui::Key::PageUp), Modifiers::ALT)),
            download: Some((KeyOrPointer::Key(egui::Key::PageDown), Modifiers::ALT)),
        }
    }
}
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
            /* TODO
            file.write_all("[KEYBINDINGS]\n".to_string().as_bytes())?;
            file.write_all(
                format!("clear_screen = \"{:?}\"\n", self.bind.clear_screen).as_bytes(),
            )?;
            */

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

fn parse_keybinding_settings(
    options: &mut Options,
    keybind_settings: &toml::map::Map<String, Value>,
) {
    /* TODO: read keybindings
    for (k, v) in keybind_settings {
        match k.as_str() {
            "clear_screen" => if let Value::String(str) = v {},
            _ => {}
        }
    }*/
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
