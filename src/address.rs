use std::{fs::{self}};
use directories::ProjectDirs;
use yaml_rust::{YamlLoader, Yaml};

pub enum Terminal {
    Ansi,
    _Avatar,
    _VT102
}

pub enum Connection {
    Telnet,
    _Rlogin,
    _SSH
}

pub struct Address {
    pub system_name: String,
    pub user_name: String,
    pub password: String,
    pub comment: String,
    pub terminal_type: Terminal,
    pub font_name: String,

    pub address: String,
    pub connection: Connection,

    pub ice_mode: bool,
}

const TEMPLATE: &str = r"
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
";

impl Address {
    pub fn new() -> Self {
        Self {
            system_name: String::new(),
            user_name: String::new(),
            password: String::new(),
            comment: String::new(),
            terminal_type: Terminal::Ansi,
            font_name: String::new(),
            address: String::new(),
            connection: Connection::Telnet,
            ice_mode: true
        }
    }

    pub fn read_phone_book() -> Vec<Self> {
        let mut res = Vec::new();

        if let Some(proj_dirs) = ProjectDirs::from("com", "GitHub",  "icy_term") {
            if !proj_dirs.config_dir().exists()
            {
                fs::create_dir(proj_dirs.config_dir()).expect("Can't create configuration directory");
            }
            let phonebook = proj_dirs.config_dir().join("phonebook.yaml");
            if !phonebook.exists()
            {
                fs::write(phonebook, &TEMPLATE).expect("Can't create phonebook");
                return res;
            }
            let fs = fs::read_to_string(phonebook).expect("Can't read phonebook");
            let data = YamlLoader::load_from_str(&fs);

            if data.is_err() {
                println!("Can't read phonebook.");
                return res;
            }
            let yaml = data.unwrap();
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
                                   _ =>  {}
                                } 
                            }
                        }
                        res.push(adr);
                    }
                }
            }
        }
        res
    }
}