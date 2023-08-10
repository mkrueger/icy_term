use crate::ui::screen_modes::ScreenMode;
use crate::TerminalResult;
use chrono::{Duration, Utc};
use directories::ProjectDirs;
use icy_engine::{
    AnsiParser, AsciiParser, AtasciiParser, AvatarParser, BufferParser, PETSCIIParser,
    ViewdataParser,
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::time::SystemTime;
use std::{
    fmt::Display,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    thread,
};
use toml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    Ansi,
    Avatar,
    Ascii,
    PETscii,
    ATAscii,
    ViewData,
}

impl Terminal {
    pub const ALL: [Terminal; 5] = [
        Terminal::Ansi,
        Terminal::Avatar,
        Terminal::Ascii,
        Terminal::PETscii,
        Terminal::ViewData,
    ];
}

impl Display for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Telnet,
    Raw,
    Ssh,
}

impl Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl ConnectionType {
    pub const ALL: [ConnectionType; 3] = [
        ConnectionType::Telnet,
        ConnectionType::Raw,
        ConnectionType::Ssh,
    ];
}

#[derive(Debug, Clone)]
pub struct AddressBook {
    pub addresses: Vec<Address>,
}

pub struct LastCall {
    pub uuid: Option<uuid::Uuid>,

    pub address: String,
    pub terminal_type: Terminal,
    pub connection_type: ConnectionType,

    pub date: Option<chrono::DateTime<Utc>>,
    pub last_call_duration: chrono::Duration,
    pub upladed_bytes: usize,
    pub downloaded_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub uuid: uuid::Uuid,
    pub system_name: String,
    pub is_favored: bool,

    pub user_name: String,
    pub password: String,
    pub comment: String,
    pub terminal_type: Terminal,

    pub address: String,
    pub auto_login: String,
    pub connection_type: ConnectionType,

    pub ice_mode: bool,
    pub ansi_music: String,

    pub font_name: Option<String>,
    pub screen_mode: ScreenMode,

    pub created: chrono::DateTime<Utc>,
    pub updated: chrono::DateTime<Utc>,
    pub overall_duration: chrono::Duration,

    pub number_of_calls: usize,
    pub last_call: Option<chrono::DateTime<Utc>>,
    pub last_call_duration: chrono::Duration,
    pub upladed_bytes: usize,
    pub downloaded_bytes: usize,
}

const TEMPLATE: &str = r#"
[[addresses]]
system_name = 'Crazy Paradise BBS'
user_name = ''
password = ''
comment = 'Last german Amiga BBS. Icy Term WHQ.'
terminal_type = 'Ansi'
address = 'cpbbs.de:2323'
auto_login = ''
connection_type = 'Telnet'
ice_mode = true
ansi_music = 'Off'

[[addresses]]
system_name = 'Deadline BBS'
user_name = ''
password = ''
comment = 'Cool BBS running PCBoard.'
terminal_type = 'Ansi'
address = 'deadline.aegis-corp.org:1337'
auto_login = ''
connection_type = 'Telnet'
ice_mode = true
ansi_music = 'Off'

[[addresses]]
system_name = 'BBS Retroacademy'
user_name = ''
password = ''
comment = 'Lovely Petscii BBS'
terminal_type = 'Ansi'
address = 'bbs.retroacademy.it:6510'
auto_login = ''
connection_type = 'Telnet'
ice_mode = true
ansi_music = 'Off'

[addresses.screen_mode]
name = 'C64'

[[addresses]]
system_name = 'Amis XE'
user_name = 'amis86'
password = 'amis86'
comment = 'Atasii id&pw: amis86'
terminal_type = 'Ansi'
address = 'amis86.ddns.net:9000'
auto_login = ''
connection_type = 'Telnet'
ice_mode = true
ansi_music = 'Off'

[addresses.screen_mode]
name = 'Atari'
"#;

impl Address {
    pub fn new(system_name: String) -> Self {
        let time = Utc::now();

        Self {
            system_name,
            user_name: String::new(),
            password: String::new(),
            comment: String::new(),
            terminal_type: Terminal::Ansi,
            font_name: None,
            screen_mode: ScreenMode::Vga(80, 25),
            auto_login: String::new(),
            address: String::new(),
            connection_type: ConnectionType::Telnet,
            ansi_music: String::new(),
            ice_mode: true,
            uuid: uuid::Uuid::new_v4(),
            is_favored: false,
            created: time,
            updated: time,
            overall_duration: Duration::zero(),
            number_of_calls: 0,
            last_call: None,
            last_call_duration: Duration::zero(),
            upladed_bytes: 0,
            downloaded_bytes: 0,
        }
    }

    pub fn get_terminal_parser(&self) -> Box<dyn BufferParser> {
        match self.terminal_type {
            Terminal::Ansi => {
                let mut parser = AnsiParser::new();
                parser.ansi_music = self.ansi_music.clone().into();
                Box::new(parser)
            }
            Terminal::Avatar => Box::new(AvatarParser::new(true)),
            Terminal::Ascii => Box::<AsciiParser>::default(),
            Terminal::PETscii => Box::<PETSCIIParser>::default(),
            Terminal::ATAscii => Box::<AtasciiParser>::default(),
            Terminal::ViewData => Box::new(ViewdataParser::new()),
        }
    }

    pub fn get_phonebook_file() -> Option<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "GitHub", "icy_term") {
            if !proj_dirs.config_dir().exists() {
                fs::create_dir_all(proj_dirs.config_dir()).unwrap_or_else(|_| {
                    panic!(
                        "Can't create configuration directory {:?}",
                        proj_dirs.config_dir()
                    )
                });
            }
            let phonebook = proj_dirs.config_dir().join("phonebook.toml");
            if !phonebook.exists() {
                fs::write(&phonebook, TEMPLATE).expect("Can't create phonebook");
            }
            return Some(phonebook);
        }
        None
    }

    pub fn read_phone_book() -> Vec<Self> {
        let mut res = Vec::new();
        res.push(Address::new(String::new()));
        if let Some(phonebook) = Address::get_phonebook_file() {
            let input_text = fs::read_to_string(&phonebook).expect("Can't read phonebook");
            let value = input_text.parse::<Value>().unwrap();
            parse_addresses(&mut res, &value);
            /* match toml::from_str::<AddressBook>(fs.as_str()) {
                Ok(addresses) => {
                    res.extend_from_slice(&addresses.addresses);
                    return res;
                }
                Err(err) => {
                    println!(
                        "Can't read phonebook from file {}: {:?}.",
                        phonebook.display(),
                        err
                    );
                    return res;
                }
            }*/
        }
        res
    }
}

pub static mut READ_ADDRESSES: bool = false;

pub fn start_read_book() -> Vec<Address> {
    let res = Address::read_phone_book();

    if let Some(phonebook) = Address::get_phonebook_file() {
        thread::spawn(move || loop {
            if let Some(path) = phonebook.parent() {
                if watch(path).is_err() {
                    return;
                }
            }
        });
    }
    res
}

pub fn store_phone_book(addr: &Vec<Address>) -> TerminalResult<()> {
    if let Some(file_name) = Address::get_phonebook_file() {
        let mut addresses = Vec::new();
        (1..addr.len()).for_each(|i| {
            addresses.push(addr[i].clone());
        });
        let phonebook = AddressBook { addresses };
        /*
        match toml::to_string_pretty(&phonebook) {
            Ok(str) => {
                let mut tmp = file_name.clone();
                if !tmp.set_extension("tmp") {
                    return Ok(());
                }
                let mut file = File::create(&tmp)?;
                file.write_all(str.as_bytes())?;
                file.sync_all()?;
                fs::rename(&tmp, file_name)?;
            }
            Err(err) => return Err(Box::new(err)),
        }*/
    }
    Ok(())
}

fn watch<P: AsRef<Path>>(path: P) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Ok(_) => unsafe {
                READ_ADDRESSES = true;
            },
            Err(e) => println!("watch error: {e:}"),
        }
    }

    Ok(())
}

fn parse_addresses(addresses: &mut Vec<Address>, value: &Value) {
    if let Value::Table(table) = value {
        let version: Option<String> = if let Some(Value::String(version)) = table.get("version") {
            Some(version.clone())
        } else {
            None
        };

        if let Some(Value::Array(values)) = table.get("addresses") {
            for value in values {
                addresses.push(parse_legacy_address(value));
            }
        }
    }
}

fn parse_legacy_address(value: &Value) -> Address {
    let mut result = Address::new(String::new());
    if let Value::Table(table) = value {
        if let Some(Value::String(value)) = table.get("system_name") {
            result.system_name = value.clone();
        }
        if let Some(Value::String(value)) = table.get("address") {
            result.address = value.clone();
        }
        if let Some(Value::String(value)) = table.get("user_name") {
            result.user_name = value.clone();
        }
        if let Some(Value::String(value)) = table.get("password") {
            result.password = value.clone();
        }
        if let Some(Value::String(value)) = table.get("comment") {
            result.comment = value.clone();
        }
        if let Some(Value::String(value)) = table.get("auto_login") {
            result.auto_login = value.clone();
        }
        if let Some(Value::String(value)) = table.get("connection_type") {
            match value.as_str() {
                "Telnet" => result.connection_type = ConnectionType::Telnet,
                "SSH" => result.connection_type = ConnectionType::Ssh,
                "Raw" => result.connection_type = ConnectionType::Raw,
                _ => {}
            }
        }

        if let Some(Value::String(value)) = table.get("terminal_type") {
            match value.as_str() {
                "Ansi" => result.terminal_type = Terminal::Ansi,
                "Avatar" => result.terminal_type = Terminal::Avatar,
                _ => {}
            }
        }

        if let Some(Value::Table(value)) = table.get("screen_mode") {
            if let Some(Value::String(name)) = value.get("name") {
                match name.as_str() {
                    "DOS" | "VT500" => {
                        result.screen_mode = ScreenMode::Vga(80, 25);
                    }
                    "C64" | "C128" => {
                        result.screen_mode = ScreenMode::Vic;
                        result.terminal_type = Terminal::PETscii;
                    }
                    "Atari" | "AtariXep80" => {
                        result.screen_mode = ScreenMode::Antic;
                        result.terminal_type = Terminal::ATAscii;
                    }
                    "Viewdata" => {
                        result.screen_mode = ScreenMode::Videotex;
                        result.terminal_type = Terminal::ViewData;
                    }
                    _ => {}
                }
            }
        }
    }

    result
}
