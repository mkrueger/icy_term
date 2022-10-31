use std::{fs::{self}, path::PathBuf, thread};
use directories::ProjectDirs;
use yaml_rust::{YamlLoader, Yaml};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Config};
use std::path::Path;

use crate::ui::screen_modes::ScreenMode;

#[derive(Debug, Clone, Copy)]
pub enum Terminal {
    Ansi,
    _Avatar,
    _VT102
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

Particles! BBS:
    comment: Particles! BBS is a retro-themed BBS, running on retro-themed hardware.
    address: particlesbbs.dyndns.org:6400
Heatwave BBS:
    comment: Heatwave runs on a vintage Myarc Geneve 9640 (TI-99/4A clone) built in 1987.
    address: heatwave.ddns.net:9640
"Piranha: Under the Black Flag":
    comment: If you love rich, colorful ANSI art
    address: blackflag.acid.org
Dark Systems BBS:
    comment: They are using Remote Access BBS.
    address: bbs.dsbbs.ca
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
            address: String::new(),
            connection: Connection::Telnet,
            ice_mode: true
        }
    }


    pub fn get_phonebook_file() -> Option<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "GitHub",  "icy_term") {
            if !proj_dirs.config_dir().exists()
            {
                fs::create_dir(proj_dirs.config_dir()).expect("Can't create configuration directory");
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
                                            "use_ice" => { adr.ice_mode = v == "true"; }
                                            "screen_mode" => { adr.screen_mode = ScreenMode::parse(&v); }
                                            "font_name" => { adr.font_name = Some(v); }
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