use crate::ui::screen_modes::ScreenMode;
use crate::TerminalResult;
use chrono::{Duration, Utc};
use icy_engine::ansi::{BaudEmulation, MusicOption};
use icy_engine::{ansi, ascii, atascii, avatar, petscii, viewdata, BufferParser};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::{
    fmt::Display,
    fs::{self},
    path::PathBuf,
};
use toml::Value;
use versions::Versioning;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Terminal {
    #[default]
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

    #[must_use]
    pub fn get_parser(&self, addr: &Address) -> Box<dyn BufferParser> {
        match self {
            Terminal::Ansi => {
                let mut parser = ansi::Parser::default();
                parser.ansi_music = addr.ansi_music;
                parser.bs_is_ctrl_char = true;
                Box::new(parser)
            }
            Terminal::Avatar => Box::<avatar::Parser>::default(),
            Terminal::Ascii => Box::<ascii::Parser>::default(),
            Terminal::PETscii => Box::<petscii::Parser>::default(),
            Terminal::ATAscii => Box::<atascii::Parser>::default(),
            Terminal::ViewData => Box::<viewdata::Parser>::default(),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Protocol {
    #[default]
    Telnet,
    Raw,
    Ssh,
    WebSocket(bool), // true=secure
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ssh => write!(f, "SSH"),
            Self::WebSocket(is_secure) => match is_secure {
                true => write!(f, "Secure WebSocket"),
                false => write!(f, "WebSocket"),
            },
            _ => write!(f, "{self:?}"),
        }
    }
}

impl Protocol {
    #[cfg(not(target_arch = "wasm32"))]
    pub const ALL: [Protocol; 5] = [
        Protocol::Telnet,
        Protocol::Raw,
        Protocol::Ssh,
        Protocol::WebSocket(true),
        Protocol::WebSocket(false),
    ];
    #[cfg(target_arch = "wasm32")]
    pub const ALL: [Protocol; 3] = [Protocol::Telnet, Protocol::Raw, Protocol::WebSocket(true)];
}

#[derive(Debug, Clone)]
pub struct AddressBook {
    pub write_lock: bool,
    created_backup: bool,
    pub addresses: Vec<Address>,
}

impl Default for AddressBook {
    fn default() -> Self {
        let mut res = Self {
            write_lock: false,
            created_backup: false,
            addresses: Vec::new(),
        };
        res.load_string(TEMPLATE).unwrap_or_default();
        res
    }
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
    pub baud_emulation: BaudEmulation,

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

    pub override_iemsi_settings: bool,
    pub iemsi_user: String,
    pub iemsi_password: String,
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

[[addresses]]
system_name = "Xibalba"
is_favored = false
address = "xibalba.l33t.codes:44510"
protocol = "Telnet"
terminal_type = "Ansi"
screen_mode = "Vga(80, 25)"
comment = "ENiGMA½ WHQ"
"#;

static mut current_id: usize = 0;

impl Address {
    pub fn new(system_name: impl Into<String>) -> Self {
        let time = Utc::now();
        unsafe {
            current_id = current_id.wrapping_add(1);
        }

        Self {
            system_name: system_name.into(),
            user_name: String::new(),
            password: String::new(),
            comment: String::new(),
            terminal_type: Terminal::default(),
            font_name: None,
            screen_mode: ScreenMode::default(),
            auto_login: String::new(),
            address: String::new(),
            protocol: Protocol::default(),
            ansi_music: MusicOption::default(),
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
            baud_emulation: BaudEmulation::default(),
            override_iemsi_settings: false,
            iemsi_user: String::new(),
            iemsi_password: String::new(),
        }
    }

    #[must_use]
    pub fn get_dialing_directory_file() -> Option<PathBuf> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "GitHub", "icy_term") {
            if !proj_dirs.config_dir().exists()
                && fs::create_dir_all(proj_dirs.config_dir()).is_err()
            {
                log::error!(
                    "Can't create configuration directory {:?}",
                    proj_dirs.config_dir()
                );
                return None;
            }
            let dialing_directory = proj_dirs.config_dir().join("phonebook.toml");
            if !dialing_directory.exists() {
                if let Err(err) = fs::write(&dialing_directory, TEMPLATE) {
                    log::error!("Can't create dialing_directory {dialing_directory:?} : {err}");
                    return None;
                }
            }
            return Some(dialing_directory);
        }
        None
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn read_phone_book() -> TerminalResult<AddressBook> {
        let mut res = AddressBook::new();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(dialing_directory) = Address::get_dialing_directory_file() {
            match fs::read_to_string(dialing_directory) {
                Ok(input_text) => {
                    if let Err(err) = res.load_string(&input_text) {
                        log::error!("Error reading phonebook {err}");
                        return Ok(AddressBook::default());
                    }
                }
                Err(err) => return Err(err.into()),
            }
        }
        Ok(res)
    }
}

pub static mut READ_ADDRESSES: bool = false;

/// .
///
/// # Errors
///
/// This function will return an error if .
pub fn start_read_book() -> TerminalResult<AddressBook> {
    let res = Address::read_phone_book()?;
    start_watch_thread();
    Ok(res)
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

impl AddressBook {
    const VERSION: &'static str = "1.1.0";

    #[must_use]
    pub fn new() -> Self {
        let addresses = vec![Address::new(String::new())];
        Self {
            write_lock: false,
            created_backup: false,
            addresses,
        }
    }

    fn load_string(&mut self, input_text: &str) -> TerminalResult<()> {
        match input_text.parse::<Value>() {
            Ok(value) => self.parse_addresses(&value),
            Err(err) => {
                return Err(format!("Error parsing dialing_directory: {err}").into());
            }
        }
        Ok(())
    }

    fn parse_addresses(&mut self, value: &Value) {
        if let Value::Table(table) = value {
            let version: Option<String> = if let Some(Value::String(version)) = table.get("version")
            {
                Some(version.clone())
            } else {
                None
            };
            if let Some(vers) = &version {
                if let Some(vers) = Versioning::new(vers) {
                    if vers > versions::Versioning::new(AddressBook::VERSION).unwrap() {
                        log::warn!("Newer address book version: {vers}");
                        self.write_lock = true;
                    }
                }
            }

            if let Some(Value::Array(values)) = table.get("addresses") {
                for value in values {
                    if version.is_some() {
                        self.addresses.push(parse_address(value));
                    } else {
                        self.addresses.push(parse_legacy_address(value));
                    }
                }
            }
        }
    }

    /// Returns the store phone book of this [`AddressBook`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn store_phone_book(&mut self) -> TerminalResult<()> {
        if self.write_lock {
            return Ok(());
        }
        if let Some(file_name) = Address::get_dialing_directory_file() {
            // create temp file to write the new dialing directory
            let mut write_name: PathBuf = file_name.clone();
            write_name.set_extension("new");
            let mut file = File::create(&write_name)?;
            file.write_all(format!("version = \"{}\"\n", AddressBook::VERSION).as_bytes())?;
            for addr in self.addresses.iter().skip(1) {
                store_address(&mut file, addr)?;
            }

            let mut backup_file: PathBuf = file_name.clone();
            backup_file.set_extension("bak");

            // Backup old file, if it has contents
            // NOTE: just backup once per session, otherwise it get's overwritten too easily.
            if !self.created_backup {
                self.created_backup = true;
                if let Ok(data) = fs::metadata(&file_name) {
                    if data.len() > 0 {
                        std::fs::rename(&file_name, &backup_file)?;
                    }
                }
            }

            // move temp file to the real file
            std::fs::rename(&write_name, &file_name)?;
        }
        Ok(())
    }
}

fn start_watch_thread() {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dialing_directory) = Address::get_dialing_directory_file() {
        if let Err(err) = std::thread::Builder::new().name("file_watcher_thread".to_string()).spawn(move || loop {
            if let Some(path) = dialing_directory.parent() {
                if watch(path).is_err() {
                    return;
                }
            }
        }) {
            log::error!("Error starting file watcher thread: {err}");
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
                "websocket(true)" => result.protocol = Protocol::WebSocket(true),
                "websocket(false)" => result.protocol = Protocol::WebSocket(false),
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

        if let Some(Value::String(value)) = table.get("baud_emulation") {
            match value.to_lowercase().as_str() {
                "off" => result.baud_emulation = BaudEmulation::Off,
                baud => {
                    let v = baud.parse::<u32>().unwrap_or(0);
                    result.baud_emulation = BaudEmulation::Rate(v);
                }
            }
        }

        if let Some(Value::String(name)) = table.get("screen_mode") {
            match name.to_lowercase().as_str() {
                "vga(80, 25)" => result.screen_mode = ScreenMode::Vga(80, 25),
                "vga(80, 50)" => result.screen_mode = ScreenMode::Vga(80, 50),
                "vga(132, 37)" => result.screen_mode = ScreenMode::Vga(132, 37),
                "vga(132, 52)" => result.screen_mode = ScreenMode::Vga(132, 52),
                "vic" => result.screen_mode = ScreenMode::Vic,
                "antic" => result.screen_mode = ScreenMode::Antic,
                "videotex" => result.screen_mode = ScreenMode::Videotex,
                _ => {}
            }
        }
        if let Some(Value::Table(map)) = table.get("IEMSI") {
            if let Some(Value::Boolean(value)) = map.get("override_settings") {
                result.override_iemsi_settings = *value;
            }
            if let Some(Value::String(value)) = map.get("user_name") {
                result.iemsi_user = value.clone();
            }
            if let Some(Value::String(value)) = map.get("password") {
                result.iemsi_password = value.clone();
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
    if addr.protocol != Protocol::default() {
        file.write_all(format!("protocol = \"{:?}\"\n", addr.protocol).as_bytes())?;
    }
    if !addr.user_name.is_empty() {
        file.write_all(format!("user_name = \"{}\"\n", escape(&addr.user_name)).as_bytes())?;
    }
    if !addr.password.is_empty() {
        file.write_all(format!("password = \"{}\"\n", escape(&addr.password)).as_bytes())?;
    }
    if !addr.auto_login.is_empty() {
        file.write_all(format!("auto_login = \"{}\"\n", escape(&addr.auto_login)).as_bytes())?;
    }

    if addr.terminal_type != Terminal::default() {
        file.write_all(format!("terminal_type = \"{:?}\"\n", addr.terminal_type).as_bytes())?;
    }

    if addr.ansi_music != MusicOption::default() {
        file.write_all(format!("ansi_music = \"{:?}\"\n", addr.ansi_music).as_bytes())?;
    }

    if addr.baud_emulation != BaudEmulation::default() {
        file.write_all(format!("baud_emulation = \"{}\"\n", addr.baud_emulation).as_bytes())?;
    }

    if addr.screen_mode != ScreenMode::default() {
        file.write_all(format!("screen_mode = \"{:?}\"\n", addr.screen_mode).as_bytes())?;
    }
    if !addr.comment.is_empty() {
        file.write_all(format!("comment = \"{}\"\n", escape(&addr.comment)).as_bytes())?;
    }
    file.write_all(format!("number_of_calls = {}\n", addr.number_of_calls).as_bytes())?;

    if let Some(last_call) = addr.last_call {
        file.write_all(format!("last_call = \"{}\"\n", last_call.to_rfc3339()).as_bytes())?;
    }
    file.write_all(format!("created = \"{}\"\n", addr.created.to_rfc3339()).as_bytes())?;

    if addr.override_iemsi_settings
        || !addr.iemsi_user.is_empty()
        || !addr.iemsi_password.is_empty()
    {
        file.write_all("[addresses.IEMSI]\n".to_string().as_bytes())?;

        if addr.override_iemsi_settings {
            file.write_all(
                format!("override_settings = {}\n", addr.override_iemsi_settings).as_bytes(),
            )?;
        }
        if !addr.iemsi_user.is_empty() {
            file.write_all(format!("user_name = \"{}\"\n", escape(&addr.iemsi_user)).as_bytes())?;
        }
        if !addr.iemsi_password.is_empty() {
            file.write_all(
                format!("password = \"{}\"\n", escape(&addr.iemsi_password)).as_bytes(),
            )?;
        }
    }

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

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::*;

    #[test]
    fn test_load_default_template() {
        let mut res = AddressBook {
            write_lock: false,
            created_backup: false,
            addresses: Vec::new(),
        };
        res.load_string(TEMPLATE).unwrap();
    }
}
