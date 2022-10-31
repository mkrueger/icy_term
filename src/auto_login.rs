use std::{io, time::{SystemTime, Duration}, thread};
use crate::{iemsi::{IEmsi}, com::Com, address::Address};

pub struct AutoLogin {
    pub logged_in: bool,
    pub iemsi: Option<IEmsi>,
    last_send: SystemTime,
    continue_time: SystemTime,

    login_expr: Vec<u8>,
    cur_expr_idx: usize,
    got_name: bool,
    name_idx: usize
}
const NAME_STR: &[u8] = b"NAME";

impl AutoLogin {

    pub fn new(login_expr: String) -> Self {
        Self {
            logged_in: false,
            iemsi: Some(IEmsi::new()),
            last_send: SystemTime::now(),
            continue_time: SystemTime::now(),
            login_expr: login_expr.as_bytes().to_vec(),
            cur_expr_idx: 0,
            name_idx: 0,
            got_name: false
        }
    }

    pub fn run_command<T: Com>(&mut self, com: &mut T, adr: &Address) -> io::Result<bool> {
        match self.login_expr[self.cur_expr_idx + 1] {
            b'D' => { // Delay for x seconds. !D4= Delay for 4 seconds
                let ch = self.login_expr[self.cur_expr_idx + 2];
                self.continue_time = self.last_send + Duration::from_secs((ch - b'0') as u64);
                self.cur_expr_idx += 3;
            }
            b'E' => { // Send cr+cr then esc + wait until other end responds
                self.cur_expr_idx += 2;
                com.write(b"\n\n\x1b")?;
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
                println!("{}", char::from_u32(ch as u32).unwrap());
                com.write(&[ch as u8])?;
                self.cur_expr_idx += 1;
            }
        }

        Ok(true)
    }

    pub fn try_login<T: Com>(&mut self, com: &mut T, adr: &Address, ch: u8) -> io::Result<()> {
        if self.logged_in {
            return Ok(());
        }
        if adr.user_name.len()  == 0 || adr.password.len() == 0 {
            self.logged_in = true;
            return Ok(());
        }
        self.last_send = SystemTime::now();
        if self.name_idx < NAME_STR.len() {
            let c = if (b'a'..b'z').contains(&ch) { ch - b'a' + b'A' } else  { ch };
            if (b'A'..b'Z').contains(&c) {
                if NAME_STR[self.name_idx] == c {
                    self.name_idx += 1;
                } else {
                    self.name_idx = 0;
                }
                self.got_name |= self.name_idx >= NAME_STR.len();
            }
        }

        if let Some(iemsi) = &mut self.iemsi {
            self.logged_in |= iemsi.try_login(com, adr, ch)?;
        }
        Ok(())
    }

    pub fn run_autologin<T: Com>(&mut self, com: &mut T, adr: &Address) -> io::Result<()> {
        if self.logged_in && self.cur_expr_idx >= self.login_expr.len() {
            return Ok(());
        }
        self.last_send = SystemTime::now();
        if self.last_send < self.continue_time {
            return Ok(());
        }
        if self.cur_expr_idx < self.login_expr.len() {
            match self.login_expr[self.cur_expr_idx] {
                b'!' => {
                    self.run_command(com, adr)?;
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