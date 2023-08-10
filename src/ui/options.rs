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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostProcessing {
    None,
    CRT1,
    CRT1CURVED,
    CRT2,
    CRT2CURVED,
}

impl PostProcessing {
    pub const ALL: [PostProcessing; 5] = [
        PostProcessing::None,
        PostProcessing::CRT1,
        PostProcessing::CRT1CURVED,
        PostProcessing::CRT2,
        PostProcessing::CRT2CURVED,
    ];
}

#[derive(Debug, Clone)]
pub struct Options {
    pub scaling: Scaling,
    pub post_processing: PostProcessing,
    pub connect_timeout: Duration,
}

impl Options {
    pub fn new() -> Self {
        Options {
            connect_timeout: Duration::from_secs(10),
            scaling: Scaling::Linear,
            post_processing: PostProcessing::CRT1,
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
            file.write_all(b"version = \"1.0\"\n")?;
            file.write_all(format!("scaling = \"{:?}\"\n", self.scaling).as_bytes())?;
            file.write_all(
                format!("post_processing = \"{:?}\"\n", self.post_processing).as_bytes(),
            )?;
            file.flush()?;
        }
        Ok(())
    }

    fn from_str(input_text: &str) -> Options {
        let value = input_text.parse::<Value>().unwrap();
        let mut result = Options::new();
        parse_value(&mut result, &value);
        return result;
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
            println!("an array");
            for v in array {
                parse_value(options, v);
            }
        }
        Value::Table(table) => {
            println!("a table");
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
                    "post_processing" => {
                        if let Value::String(str) = v {
                            match str.as_str() {
                                "None" => options.post_processing = PostProcessing::None,
                                "CRT1" => options.post_processing = PostProcessing::CRT1,
                                "CRT1CURVED" => {
                                    options.post_processing = PostProcessing::CRT1CURVED
                                }
                                "CRT2" => options.post_processing = PostProcessing::CRT2,
                                "CRT2CURVED" => {
                                    options.post_processing = PostProcessing::CRT2CURVED
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
