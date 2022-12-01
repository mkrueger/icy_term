use crate::{address::Address, auto_file_transfer::PatternRecognizer, iemsi::IEmsi, com::Connection, TerminalResult};
use std::{
    io::{self, ErrorKind},
    time::{Duration, SystemTime},
};

pub struct AutoLogin {
    pub logged_in: bool,
    pub disabled: bool,
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
            disabled: false,
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

    pub fn run_command(&mut self, con: &mut Connection, adr: &Address) -> TerminalResult<bool> {
        match self.login_expr[self.cur_expr_idx + 1] {
            b'D' => {
                // Delay for x seconds. !D4= Delay for 4 seconds
                let ch = self.login_expr[self.cur_expr_idx + 2];
                self.continue_time = self.last_char_recv + Duration::from_secs((ch - b'0') as u64);
                self.cur_expr_idx += 3;
            }
            b'E' => {
                // wait until data came in
                match self.first_char_recv {
                    Some(_) => {
                        if SystemTime::now().duration_since(self.last_char_recv).unwrap().as_millis() < 500 {
                            return Ok(true);
                        }
                    }
                    _ => return Ok(true)
                }
                self.cur_expr_idx += 2;
                return Ok(true);
            }
            b'W' => {
                // Wait for one of the name questions defined arrives
                if self.got_name {
                    self.cur_expr_idx += 2;
                }
                return Ok(false);
            }
            b'N' => {
                // Send full user name of active user
                self.cur_expr_idx += 2;
                con.send((adr.user_name.clone() + "\r").as_bytes().to_vec())?;
            }
            b'F' => {
                // Send first name of active user
                self.cur_expr_idx += 2;
                con.send((adr.user_name.clone() + "\r").as_bytes().to_vec())?;
                // TODO
            }
            b'L' => {
                // Send last name of active user
                self.cur_expr_idx += 2;
                con.send((adr.user_name.clone() + "\r").as_bytes().to_vec())?;
                // TODO
            }
            b'P' => {
                // Send password from active user
                self.cur_expr_idx += 2;
                con.send((adr.password.clone() + "\r").as_bytes().to_vec())?;
                self.logged_in = true;
            }
            b'I' => {
                // Disable IEMSI in this session
                self.cur_expr_idx += 2;
                if let Some(iemsi) = &mut self.iemsi {
                    iemsi.aborted = true;
                }
            }
            ch => {
                con.send(vec![ch as u8])?;
                self.cur_expr_idx += 1;
            }
        }

        Ok(true)
    }

    pub fn try_login(&mut self, con: &mut Connection, adr: &Address, ch: u8) -> TerminalResult<()> {
        if self.logged_in || self.disabled {
            return Ok(());
        }
        if adr.user_name.len() == 0 || adr.password.len() == 0 {
            self.logged_in = true;
            return Ok(());
        }

        if self.first_char_recv.is_none() {
            if (b'A'..b'Z').contains(&ch) || (b'a'..b'z').contains(&ch) {
                self.first_char_recv = Some(SystemTime::now());
            }
        }

        self.last_char_recv = SystemTime::now();
        self.got_name |= self.name_recognizer.push_ch(ch) | self.login_recognizer.push_ch(ch);

        if let Some(iemsi) = &mut self.iemsi {
            self.logged_in |= iemsi.try_login(con, adr, ch)?;
        }
        Ok(())
    }

    pub fn run_autologin(&mut self, con: &mut Connection,  adr: &Address) -> TerminalResult<()> {
        if self.logged_in && self.cur_expr_idx >= self.login_expr.len() || self.disabled {
            return Ok(());
        }
        if adr.user_name.len() == 0 || adr.password.len() == 0 {
            self.logged_in = true;
            return Ok(());
        }

        if adr.auto_login.len() == 0 {
            return Ok(());
        }

        if SystemTime::now() < self.continue_time {
            return Ok(());
        }
        if self.cur_expr_idx < self.login_expr.len() {
            match self.login_expr[self.cur_expr_idx] {
                b'!' => {
                    self.run_command(con, adr)?;
                }
                b'\\' => {
                    while self.cur_expr_idx < self.login_expr.len() && self.login_expr[self.cur_expr_idx] == b'\\' {
                        self.cur_expr_idx += 1; // escape
                        match self.login_expr[self.cur_expr_idx] {
                            b'e' => {
                                println!("send esc!");
                                con.send(vec![0x1B])?;
                            }
                            b'n' => {
                                println!("send lf!");
                                con.send(vec![b'\n'])?;
                            }
                            b'r' => {
                                println!("send cr!");
                                con.send(vec![b'\r'])?;
                            }
                            b't' => {
                                println!("send tab!");
                                con.send(vec![b'\t'])?;
                            }
                            ch => {
                                println!("send char {}!", ch);
                                self.cur_expr_idx += 1; // escape
                                return Err(Box::new(io::Error::new(
                                    ErrorKind::InvalidData,
                                    format!(
                                        "invalid escape sequence in autologin string: {:?}",
                                        char::from_u32(ch as u32)
                                    ),
                                )));
                            }
                        }
                        self.cur_expr_idx += 1; // escape
                    }
                    self.last_char_recv = SystemTime::now();

                }
                ch => {
                    con.send(vec![ch])?;
                    self.cur_expr_idx += 1;
                }
            }
        }
        Ok(())
    }
}
