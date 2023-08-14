use std::{
    fs::{self, File},
    io::Write,
    time::Duration,
};

use directories::ProjectDirs;
use toml::Value;

use crate::TerminalResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scaling {
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

#[derive(Debug, Clone)]
pub struct Options {
    pub scaling: Scaling,
    pub connect_timeout: Duration,
    pub monitor_settings: MonitorSettings,
}

impl Options {
    pub fn new() -> Self {
        Options {
            connect_timeout: Duration::from_secs(10),
            scaling: Scaling::Linear,
            monitor_settings: MonitorSettings::default(),
        }
    }

    pub fn load_options() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("com", "GitHub", "icy_term") {
            let options_file = proj_dirs.config_dir().join("options.toml");
            if options_file.exists() {
                let fs = fs::read_to_string(&options_file).expect("Can't read options");
                return Options::from_str(&fs);
            }
        }
        Options::new()
    }

    pub fn store_options(&self) -> TerminalResult<()> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "GitHub", "icy_term") {
            let options_file = proj_dirs.config_dir().join("options.toml");

            let mut file = File::create(options_file)?;
            file.write_all(b"version = \"1.1\"\n")?;

            file.write_all(format!("scaling = \"{:?}\"\n", self.scaling).as_bytes())?;
            file.write_all(
                format!(
                    "use_crt_filter = \"{:?}\"\n",
                    self.monitor_settings.use_filter
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_type = \"{:?}\"\n",
                    self.monitor_settings.monitor_type
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!("monitor_gamma = \"{:?}\"\n", self.monitor_settings.gamma).as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_contrast = \"{:?}\"\n",
                    self.monitor_settings.contrast
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_saturation = \"{:?}\"\n",
                    self.monitor_settings.saturation
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_brightness = \"{:?}\"\n",
                    self.monitor_settings.brightness
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!("monitor_blur = \"{:?}\"\n", self.monitor_settings.blur).as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_curvature = \"{:?}\"\n",
                    self.monitor_settings.curvature
                )
                .as_bytes(),
            )?;
            file.write_all(
                format!(
                    "monitor_scanlines = \"{:?}\"\n",
                    self.monitor_settings.scanlines
                )
                .as_bytes(),
            )?;
            file.flush()?;
        }
        Ok(())
    }

    fn from_str(input_text: &str) -> Options {
        let value = input_text.parse::<Value>().unwrap();
        let mut result = Options::new();
        parse_value(&mut result, &value);
        result
    }
}

fn parse_value(options: &mut Options, value: &Value) {
    match value {
        Value::String(string) => {
            println!("a string --> {string}");
        }
        Value::Integer(integer) => {
            println!("an integer --> {integer}");
        }
        Value::Float(float) => {
            println!("a float --> {float}");
        }
        Value::Boolean(boolean) => {
            println!("a boolean --> {boolean}");
        }
        Value::Datetime(datetime) => {
            println!("a datetime --> {datetime}");
        }
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
                    _ => {}
                }
            }
        }
    }
}
