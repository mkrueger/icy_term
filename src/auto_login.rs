use std::{io::{self, ErrorKind}, time::{SystemTime, Duration}};
use crate::{iemsi::{IEmsi}, com::Com, address::Address, auto_file_transfer::PatternRecognizer};

pub struct AutoLogin {
    pub logged_in: bool,
    pub iemsi: Option<IEmsi>,
    last_char_recv: SystemTime,
    first_char_recv: Option<SystemTime>,
    continue_time: SystemTime,

    login_expr: Vec<u8>,
    cur_expr_idx: usize,
    got_name: bool,
    name_recognizer: PatternRecognizer,
    login_recognizer: PatternRecognizer,
}

impl AutoLogin {

    pub fn new(login_expr: String) -> Self {
        Self {
            logged_in: false,
            iemsi: Some(IEmsi::new()),
            first_char_recv: None,
            last_char_recv: SystemTime::now(),
            continue_time: SystemTime::now(),
            login_expr: login_expr.as_bytes().to_vec(),
            cur_expr_idx: 0,
            got_name: false,
            name_recognizer: PatternRecognizer::from(b"NAME", true),
            login_recognizer: PatternRecognizer::from(b"LOGIN:", true),
        }
    }

    pub fn run_command(&mut self, com: &mut Box<dyn Com>, adr: &Address) -> io::Result<bool> {
        match self.login_expr[self.cur_expr_idx + 1] {
            b'D' => { // Delay for x seconds. !D4= Delay for 4 seconds
                let ch = self.login_expr[self.cur_expr_idx + 2];
                self.continue_time = self.last_char_recv + Duration::from_secs((ch - b'0') as u64);
                self.cur_expr_idx += 3;
            }
            b'E' => { // wait until data came in
                match self.first_char_recv {
                    Some(t) => {
                        if SystemTime::now().duration_since(t).unwrap().as_millis() < 500 {
                            return Ok(true);
                        }
                        self.first_char_recv = None;
                        self.cur_expr_idx += 2;
                    }
                    _ => {}
                }
                return Ok(true);
            }
            b'W' => { // Wait for one of the name questions defined arrives
                if self.got_name {
                    self.cur_expr_idx += 2;
                }
                return Ok(false);
            }
            b'N' => { // Send full user name of active user
                self.cur_expr_idx += 2;
                com.write((adr.user_name.clone() + "\r").as_bytes())?;
            }
            b'F' => { // Send first name of active user
                self.cur_expr_idx += 2;
                com.write((adr.user_name.clone() + "\r").as_bytes())?;
                // TODO
            }
            b'L' => { // Send last name of active user
                self.cur_expr_idx += 2;
                com.write((adr.user_name.clone() + "\r").as_bytes())?;
                // TODO
            }
            b'P' => { // Send password from active user
                self.cur_expr_idx += 2;
                com.write((adr.password.clone() + "\r").as_bytes())?;
                self.logged_in = true;
            }
            b'I' => { // Disable IEMSI in this session
                self.cur_expr_idx += 2;
                if let Some(iemsi) = &mut self.iemsi {
                    iemsi.aborted = true;
                }
            }
            ch => {
                com.write(&[ch as u8])?;
                self.cur_expr_idx += 1;
            }
        }

        Ok(true)
    }

    pub fn try_login(&mut self, com: &mut Box<dyn Com>, adr: &Address, ch: u8) -> io::Result<()> {
        if self.logged_in {
            return Ok(());
        }
        if adr.user_name.len()  == 0 || adr.password.len() == 0 {
            self.logged_in = true;
            return Ok(());
        }

        if self.first_char_recv.is_none() {
            if (b'A'..b'Z').contains(&ch)  || (b'a'..b'z').contains(&ch) {
                self.first_char_recv = Some(SystemTime::now());
            }
        }

        self.last_char_recv = SystemTime::now();
        self.got_name |= self.name_recognizer.push_ch(ch) |  self.login_recognizer.push_ch(ch);

        if let Some(iemsi) = &mut self.iemsi {
            self.logged_in |= iemsi.try_login(com, adr, ch)?;
        }
        Ok(())
    }

    pub fn run_autologin(&mut self, com: &mut Box<dyn Com>, adr: &Address) -> io::Result<()> {
        if self.logged_in && self.cur_expr_idx >= self.login_expr.len() {
            return Ok(());
        }
        if adr.user_name.len()  == 0 || adr.password.len() == 0|| adr.auto_login.len() == 0 {
            self.logged_in = true;
            return Ok(());
        }

        if adr.auto_login.len() == 0 {
            return Ok(());
        }

        self.last_char_recv = SystemTime::now();
        if self.last_char_recv < self.continue_time {
            return Ok(());
        }
        if self.cur_expr_idx < self.login_expr.len() {
            match self.login_expr[self.cur_expr_idx] {
                b'!' => {
                    self.run_command(com, adr)?;
                }
                b'\\' => {
                    self.cur_expr_idx += 1; // escape 
                    match self.login_expr[self.cur_expr_idx] {
                        b'e' => { com.write(&[b'\x1B'])?; println!("send escape"); } , 
                        b'n' => { com.write(&[b'\n'])?; } ,
                        b'r' => { com.write(&[b'\r'])?; println!("send return");  } ,
                        b't' => { com.write(&[b'\t'])?; } ,
                        ch => {
                            self.cur_expr_idx += 1; // escape 
                            return Err(io::Error::new(ErrorKind::InvalidData, format!("invalid escape sequence in autologin string: {:?}", char::from_u32(ch as u32))));
                        }
                    }

                    self.cur_expr_idx += 1; // escape 

                }
                ch => {
                    com.write(&[ch])?;
                    self.cur_expr_idx += 1;
                }
            }
        }
        Ok(())
    }

}