use std::{fs::{self, File}, path::{PathBuf}, thread, fmt::Display, io::{self, Write}};
use directories::ProjectDirs;
use icy_engine::{BufferParser, AnsiParser, AvatarParser, PETSCIIParser};
use yaml_rust::{YamlLoader, Yaml};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config};
use std::path::Path;

use crate::ui::screen_modes::ScreenMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    Ansi,
    Avatar
}
impl Terminal {
    pub const ALL: [Terminal;2] = [
        Terminal::Ansi,
        Terminal::Avatar
    ];
}

impl Display for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Connection {
    Telnet,
    _Rlogin,
    _SSH
}

#[derive(Debug, Clone)]
pub struct Address {
    pub system_name: String,
    pub user_name: String,
    pub password: String,
    pub comment: String,
    pub terminal_type: Terminal,

    pub address: String,
    pub auto_login: String,
    pub connection: Connection,

    pub ice_mode: bool,

    pub font_name: Option<String>,
    pub screen_mode: Option<ScreenMode>,
}

const TEMPLATE: &str = r#"
# 
# Cool BBS:
#     comment: Some description
#     address: host:23
#     user: my_name
#     password: my_pw
#     use_ice: true
# Cool BBS #2:
#     comment: Some description
#     address: other:23
#     user: my_name
#     password: my_pw_which_is_totally_different_from_that_above
#     use_ice: true

# "screen_mode" support: "C64", "C128", "C128#80", "Atari", "AtariXep80", [row]x[col]
# font support via "font_name" - screen modes set the correct font
# Amiga fonts:
# "Amiga Topaz 1", "Amiga Topaz 1+", "Amiga Topaz 2", "Amiga Topaz 2+"
# "Amiga P0T-NOoDLE"
# "Amiga MicroKnight", "Amiga MicroKnight+"
# "Amiga mOsOul"

Crazy Paradise BBS:
    comment: Last Amiga BBS in germany
    address: cpbbs.de:2323
Deadline BBS:
    comment: One of the coolest looking PCboard systems I've ever seen.
    address: deadline.aegis-corp.org:1337   
BBS Retroacademy:
    comment: Petsci BBS.
    address: bbs.retroacademy.it:6510
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
            connection: Connection::Telnet,
            ice_mode: true
        }
    }

    pub fn get_terminal_parser(&self) -> Box<dyn BufferParser> {

        match self.screen_mode {
            Some(ScreenMode::C64)|  Some(ScreenMode::C128(_)) => {
                return Box::new(PETSCIIParser::new());
            }
            _ => {}
        }

        match self.terminal_type {
            Terminal::Avatar => Box::new(AvatarParser::new(true)),
            _ => Box::new(AnsiParser::new()),
        }
    }

    pub fn get_phonebook_file() -> Option<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "GitHub",  "icy_term") {
            if !proj_dirs.config_dir().exists()
            {
                fs::create_dir_all(proj_dirs.config_dir()).expect(&format!("Can't create configuration directory {:?}", proj_dirs.config_dir()));
            }
            let phonebook = proj_dirs.config_dir().join("phonebook.yaml");
            if !phonebook.exists()
            {
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
            let data = YamlLoader::load_from_str(&fs);
            match data {
                Ok(yaml) => {
                    for adr in yaml {
                        if let Yaml::Hash(h) = adr {
                            for (k, v) in h {
                                let mut adr = Address::new();
                                adr.system_name = k.into_string().unwrap();

                                if let Yaml::Hash(h) = v {
                                    for (k, v) in h {
                                        let k  = k.into_string().unwrap();
                                        let v  = v.into_string().unwrap();
                                        match k.as_ref() {
                                            "comment" => { adr.comment = v; }
                                            "address" => { adr.address = v; }
                                            "user" => { adr.user_name = v; }
                                            "password" => { adr.password = v; }
                                            "auto_login" => { adr.auto_login = v; }
                                            "use_ice" => { adr.ice_mode = v == "true"; }
                                            "screen_mode" => { adr.screen_mode = ScreenMode::parse(&v); }
                                            "font_name" => { adr.font_name = Some(v); }
                                            "terminal" => { adr.terminal_type = if v.to_uppercase() == "ANSI" { Terminal::Ansi } else { Terminal::Avatar } }
                                        _ =>  {}
                                        } 
                                    }
                                }
                                res.push(adr);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Can't read phonebook from file {}: {:?}.", phonebook.display(), err);
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
        let p  = phonebook.clone();
        thread::spawn(move || {
            loop {
                if let Some(path) = p.parent() {
                    if let Err(_) = watch(path) {
                        return;
                    }
                }
            }
        });
    }
    res
}

fn escape(str: &str) -> String
{
    let mut result = String::new();
    result.push('"');
    for c in str.chars() {

        if c < ' ' || c > '\x7F' {
            if c == '\\'  {
                result.push_str("\\\\");
            } else if c == '\n'  {
                result.push_str("\\n");
            } else if c == '\r'  { 
                result.push_str("\\r");
            } else { 
                result.push_str(&format!("\\x{:02X}", c as u8));
            }
        } else {
            if c == '"' {
                result.push_str("\\\"");
            } else {
                result.push(c);
            }
        }
    }
    result.push('"');
    result 
}

pub fn store_phone_book(addr: &Vec<Address>) -> io::Result<()> {
    if let Some(file_name) = Address::get_phonebook_file() {
        let mut tmp = file_name.clone();
        if !tmp.set_extension("tmp") { 
            return Ok(());
        }
        {
            let mut file = File::create(&tmp)?;

            for entry in &addr[1..] {
                file.write(format!("{}:\n", entry.system_name).as_bytes())?;
                file.write(format!("   address: {}\n", escape(&entry.address)).as_bytes())?;
                file.write(format!("   user: {}\n", escape(&entry.user_name)).as_bytes())?;
                file.write(format!("   password: {}\n", escape(&entry.password)).as_bytes())?;
                file.write(format!("   comment: {}\n", escape(&entry.comment)).as_bytes())?;
                file.write(format!("   terminal: {}\n", escape(&entry.terminal_type.to_string())).as_bytes())?;
                file.write(format!("   auto_login: {}\n", escape(&entry.auto_login)).as_bytes())?;
                if let Some(screen_mode) = &entry.screen_mode {
                    file.write(format!("   screen_mode: \"{}\"\n", screen_mode).as_bytes())?;
                }
                file.write(b"\n")?;
            }
            file.sync_all()?;
        }
        fs::rename(&tmp, file_name)?;
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
            Ok(_) => unsafe { READ_ADDRESSES = true; },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}