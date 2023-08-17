use crate::ui::screen_modes::ScreenMode;
use crate::ui::AddressCategory;
use crate::TerminalResult;
use chrono::{Duration, Utc};
use directories::ProjectDirs;
use icy_engine::ansi::{MusicOption, BaudOption};
use icy_engine::{ansi, ascii, atascii, avatar, petscii, viewdata, BufferParser};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{
    fmt::Display,
    fs::{self},
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
    pub const ALL: [Terminal; 6] = [
        Terminal::Ansi,
        Terminal::Avatar,
        Terminal::Ascii,
        Terminal::PETscii,
        Terminal::ATAscii,
        Terminal::ViewData,
    ];
}

impl Display for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Terminal::Ansi => write!(f, "ANSI"),
            Terminal::Avatar => write!(f, "AVATAR"),
            Terminal::Ascii => write!(f, "Raw (ASCII)"),
            Terminal::PETscii => write!(f, "PETSCII"),
            Terminal::ATAscii => write!(f, "ATASCII"),
            Terminal::ViewData => write!(f, "VIEWDATA"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Telnet,
    Raw,
    Ssh,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Protocol {
    pub const ALL: [Protocol; 2] = [Protocol::Telnet, Protocol::Raw];
    //pub const ALL: [Protocol; 3] = [Protocol::Telnet, Protocol::Raw, Protocol::Ssh];
}

#[derive(Debug, Clone)]
pub struct AddressBook {
    pub addresses: Vec<Address>,
}
/*
pub struct LastCall {
    pub uuid: Option<uuid::Uuid>,

    pub address: String,
    pub terminal_type: Terminal,
    pub connection_type: ConnectionType,

    pub date: Option<chrono::DateTime<Utc>>,
    pub last_call_duration: chrono::Duration,
    pub uploaded_bytes: usize,
    pub downloaded_bytes: usize,
}*/

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub id: usize,
    pub system_name: String,
    pub is_favored: bool,

    pub user_name: String,
    pub password: String,
    pub comment: String,
    pub terminal_type: Terminal,

    pub address: String,
    pub auto_login: String,
    pub protocol: Protocol,

    pub ice_mode: bool,
    pub ansi_music: MusicOption,
    pub baud_emulation: BaudOption,

    pub font_name: Option<String>,
    pub screen_mode: ScreenMode,

    pub created: chrono::DateTime<Utc>,
    pub updated: chrono::DateTime<Utc>,
    pub overall_duration: chrono::Duration,

    pub number_of_calls: usize,
    pub last_call: Option<chrono::DateTime<Utc>>,
    pub last_call_duration: chrono::Duration,
    pub uploaded_bytes: usize,
    pub downloaded_bytes: usize,

    // UI
    pub address_category: AddressCategory,
}

const TEMPLATE: &str = r#"
version = "1.0"

[[addresses]]
system_name = "Crazy Paradise BBS"
is_favored = false
address = "cpbbs.de:2323"
protocol = "Telnet"
terminal_type = "Ansi"
screen_mode = "Vga(80, 25)"
comment = "Last german Amiga BBS. Icy Term WHQ."

[[addresses]]
system_name = "BBS Retrocampus"
is_favored = false
address = "BBS.RETROCAMPUS.COM:6510"
protocol = "Telnet"
terminal_type = "PETscii"
screen_mode = "Vic"
comment = "Lovely Petscii BBS"

[[addresses]]
system_name = "Amis XE"
is_favored = false
address = "amis86.ddns.net:9000"
protocol = "Telnet"
terminal_type = "ATAscii"
screen_mode = "Antic"
comment = "Atasii id&pw: amis86"

[[addresses]]
system_name = "ntxtel"
is_favored = false
address = "nx.nxtel.org:23280"
protocol = "Telnet"
terminal_type = "ViewData"
screen_mode = "Videotex"
comment = "ViewData BBS"

[[addresses]]
system_name = "dura-bbs.net:6359"
address = "dura-bbs.net:6359"
protocol = "Telnet"
terminal_type = "Ansi"
screen_mode = "Vga(80, 25)"
"#;

static mut current_id: usize = 0;

impl Address {
    pub fn new(system_name: String) -> Self {
        let time = Utc::now();
        unsafe {
            current_id = current_id.wrapping_add(1);
        }

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
            protocol: Protocol::Telnet,
            ansi_music: MusicOption::Off,
            ice_mode: true,
            id: unsafe { current_id },
            is_favored: false,
            created: time,
            updated: time,
            overall_duration: Duration::zero(),
            number_of_calls: 0,
            last_call: None,
            last_call_duration: Duration::zero(),
            uploaded_bytes: 0,
            downloaded_bytes: 0,
            address_category: AddressCategory::Server,
            baud_emulation: BaudOption::Off,
        }
    }

    pub fn get_terminal_parser(&self, addr: &Address) -> Box<dyn BufferParser> {
        match self.terminal_type {
            Terminal::Ansi => {
                let mut parser = ansi::Parser::default();
                parser.ansi_music = addr.ansi_music;
                Box::new(parser)
            }
            Terminal::Avatar => Box::<avatar::Parser>::default(),
            Terminal::Ascii => Box::<ascii::Parser>::default(),
            Terminal::PETscii => Box::<petscii::Parser>::default(),
            Terminal::ATAscii => Box::<atascii::Parser>::default(),
            Terminal::ViewData => Box::<viewdata::Parser>::default(),
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
            let input_text = fs::read_to_string(phonebook).expect("Can't read phonebook");
            match input_text.parse::<Value>() {
                Ok(value) => parse_addresses(&mut res, &value),
                Err(err) => {
                    eprintln!("Error parsing phonebook: {err}");
                }
            }
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

pub fn store_phone_book(addresses: &[Address]) -> TerminalResult<()> {
    if let Some(file_name) = Address::get_phonebook_file() {
        let mut file = File::create(file_name)?;
        file.write_all(b"version = \"1.0\"\n")?;

        for addr in addresses.iter().skip(1) {
            store_address(&mut file, addr)?;
        }
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
                if version.is_some() {
                    addresses.push(parse_address(value));
                } else {
                    addresses.push(parse_legacy_address(value));
                }
            }
        }
    }
}

fn parse_address(value: &Value) -> Address {
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
        if let Some(Value::Boolean(value)) = table.get("is_favored") {
            result.is_favored = *value;
        }

        if let Some(Value::Integer(value)) = table.get("number_of_calls") {
            if *value >= 0 {
                result.number_of_calls = *value as usize;
            }
        }

        if let Some(Value::String(value)) = table.get("last_call") {
            result.last_call = Some(chrono::DateTime::parse_from_rfc3339(value).unwrap().into());
        }

        if let Some(Value::String(value)) = table.get("created") {
            result.created = chrono::DateTime::parse_from_rfc3339(value).unwrap().into();
        }

        if let Some(Value::String(value)) = table.get("protocol") {
            match value.to_lowercase().as_str() {
                "telnet" => result.protocol = Protocol::Telnet,
                "ssh" => result.protocol = Protocol::Ssh,
                "raw" => result.protocol = Protocol::Raw,
                _ => {}
            }
        }

        if let Some(Value::String(value)) = table.get("ansi_music") {
            match value.to_lowercase().as_str() {
                "banana" => result.ansi_music = MusicOption::Banana,
                "conflicting" => result.ansi_music = MusicOption::Conflicting,
                "both" => result.ansi_music = MusicOption::Both,
                _ => {}
            }
        }

        if let Some(Value::String(value)) = table.get("terminal_type") {
            match value.to_lowercase().as_str() {
                "ansi" => result.terminal_type = Terminal::Ansi,
                "avatar" => result.terminal_type = Terminal::Avatar,
                "ascii" => result.terminal_type = Terminal::Ascii,
                "petscii" => result.terminal_type = Terminal::PETscii,
                "atascii" => result.terminal_type = Terminal::ATAscii,
                "viewdata" => result.terminal_type = Terminal::ViewData,
                _ => {}
            }
        }

        if let Some(Value::String(name)) = table.get("screen_mode") {
            match name.to_lowercase().as_str() {
                "vga(80, 25)" => result.screen_mode = ScreenMode::Vga(80, 25),
                "vga(80, 50)" => result.screen_mode = ScreenMode::Vga(80, 50),
                "vic" => result.screen_mode = ScreenMode::Vic,
                "antic" => result.screen_mode = ScreenMode::Antic,
                "videotex" => result.screen_mode = ScreenMode::Videotex,
                _ => {}
            }
        }
    }

    result
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn store_address(file: &mut File, addr: &Address) -> TerminalResult<()> {
    file.write_all(b"\n[[addresses]]\n")?;
    file.write_all(format!("system_name = \"{}\"\n", escape(&addr.system_name)).as_bytes())?;
    if addr.is_favored {
        file.write_all(format!("is_favored = {}\n", addr.is_favored).as_bytes())?;
    }
    file.write_all(format!("address = \"{}\"\n", escape(&addr.address)).as_bytes())?;
    file.write_all(format!("protocol = \"{:?}\"\n", addr.protocol).as_bytes())?;
    if !addr.user_name.is_empty() {
        file.write_all(format!("user_name = \"{}\"\n", escape(&addr.user_name)).as_bytes())?;
    }
    if !addr.password.is_empty() {
        file.write_all(format!("password = \"{}\"\n", escape(&addr.password)).as_bytes())?;
    }
    if !addr.auto_login.is_empty() {
        file.write_all(format!("auto_login = \"{}\"\n", escape(&addr.auto_login)).as_bytes())?;
    }
    file.write_all(format!("terminal_type = \"{:?}\"\n", addr.terminal_type).as_bytes())?;
    if addr.ansi_music != MusicOption::Off {
        file.write_all(format!("ansi_music = \"{:?}\"\n", addr.ansi_music).as_bytes())?;
    }
    file.write_all(format!("screen_mode = \"{:?}\"\n", addr.screen_mode).as_bytes())?;
    if !addr.comment.is_empty() {
        file.write_all(format!("comment = \"{}\"\n", escape(&addr.comment)).as_bytes())?;
    }
    file.write_all(format!("number_of_calls = {}\n", addr.number_of_calls).as_bytes())?;

    if let Some(last_call) = addr.last_call {
        file.write_all(format!("last_call = \"{}\"\n", last_call.to_rfc3339()).as_bytes())?;
    }
    file.write_all(format!("created = \"{}\"\n", addr.created.to_rfc3339()).as_bytes())?;
    Ok(())
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
                "Telnet" => result.protocol = Protocol::Telnet,
                "SSH" => result.protocol = Protocol::Ssh,
                "Raw" => result.protocol = Protocol::Raw,
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
