use crate::ui::screen_modes::ScreenMode;
use directories::ProjectDirs;
use icy_engine::{AnsiParser, AtasciiParser, AvatarParser, BufferParser, PETSCIIParser, ViewdataParser};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::{
    error::Error,
    fmt::Display,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    thread,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terminal {
    Ansi,
    Avatar,
}

impl Terminal {
    pub const ALL: [Terminal; 2] = [Terminal::Ansi, Terminal::Avatar];
}

impl Display for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionType {
    Telnet,
    Raw,
    SSH
}

impl Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ConnectionType {
    pub const ALL: [ConnectionType; 2] = [ConnectionType::Telnet, ConnectionType::Raw];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBook {
    pub addresses: Vec<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub system_name: String,
    pub user_name: String,
    pub password: String,
    pub comment: String,
    pub terminal_type: Terminal,

    pub address: String,
    pub auto_login: String,
    pub connection_type: ConnectionType,

    pub ice_mode: bool,

    #[serde(default)]
    pub ansi_music: String,

    pub font_name: Option<String>,
    pub screen_mode: Option<ScreenMode>,
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

[addresses.screen_mode]
name = 'Atari'
"#;

impl Address {
    pub fn new() -> Self {
        Self {
            system_name: String::new(),
            user_name: String::new(),
            password: String::new(),
            comment: String::new(),
            terminal_type: Terminal::Ansi,
            font_name: None,
            screen_mode: None,
            auto_login: String::new(),
            address: String::new(),
            connection_type: ConnectionType::Telnet,
            ansi_music: String::new(),
            ice_mode: true,
        }
    }

    pub fn get_terminal_parser(&self) -> Box<dyn BufferParser> {
        match self.screen_mode {
            Some(ScreenMode::C64) | Some(ScreenMode::C128(_)) => {
                return Box::new(PETSCIIParser::new());
            }
            Some(ScreenMode::Atari) | Some(ScreenMode::AtariXep80) => {
                return Box::new(AtasciiParser::new());
            }
            Some(ScreenMode::Viewdata) => {
                return Box::new(ViewdataParser::new());
            }
            _ => {}
        }

        match self.terminal_type {
            Terminal::Avatar => Box::new(AvatarParser::new(true)),
            _ => {
                let mut parser = AnsiParser::new();
                parser.ansi_music = self.ansi_music.clone().into();
                Box::new(parser)
            },
        }
    }

    pub fn get_phonebook_file() -> Option<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "GitHub", "icy_term") {
            if !proj_dirs.config_dir().exists() {
                fs::create_dir_all(proj_dirs.config_dir()).expect(&format!(
                    "Can't create configuration directory {:?}",
                    proj_dirs.config_dir()
                ));
            }
            let phonebook = proj_dirs.config_dir().join("phonebook.toml");
            if !phonebook.exists() {
                fs::write(phonebook, &TEMPLATE).expect("Can't create phonebook");
                return None;
            }
            return Some(phonebook);
        }
        None
    }

    pub fn read_phone_book() -> Vec<Self> {
        let mut res = Vec::new();
        res.push(Address::new());
        if let Some(phonebook) = Address::get_phonebook_file() {
            let fs = fs::read_to_string(&phonebook).expect("Can't read phonebook");
            match toml::from_str::<AddressBook>(&fs.as_str()) {
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
            }
        }
        res
    }
}

pub static mut READ_ADDRESSES: bool = false;

pub fn start_read_book() -> Vec<Address> {
    let res = Address::read_phone_book();

    if let Some(phonebook) = Address::get_phonebook_file() {
        let p = phonebook.clone();
        thread::spawn(move || loop {
            if let Some(path) = p.parent() {
                if let Err(_) = watch(path) {
                    return;
                }
            }
        });
    }
    res
}

pub fn store_phone_book(addr: &Vec<Address>) -> Result<(), Box<dyn Error>> {
    if let Some(file_name) = Address::get_phonebook_file() {
        let mut addresses = Vec::new();
        for i in 1..addr.len() {
            addresses.push(addr[i].clone());
        }
        let phonebook = AddressBook { addresses };

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
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
