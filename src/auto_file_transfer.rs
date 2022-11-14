use crate::{ protocol::{ProtocolType}};

pub struct PatternRecognizer {
    pattern: Vec<u8>,
    cur_idx: usize,
    ignore_case: bool
}

impl PatternRecognizer {
    pub fn from(data: &[u8], ignore_case: bool) -> Self {
        Self {
            pattern: if ignore_case { data.iter().map(|c| to_upper(*c)).collect() } else { data.to_vec()},
            cur_idx: 0,
            ignore_case
        }
    }

    pub fn reset(&mut self) {
        self.cur_idx = 0;
    }

    pub fn push_ch(&mut self, ch: u8) ->bool {
        let p = self.pattern[self.cur_idx];
        if p == ch || self.ignore_case && ch >= b'a' && ch <= b'z' && ch  + (b'a' - b'A') == p {
            self.cur_idx += 1;
            if self.cur_idx >= self.pattern.len() {
                self.cur_idx = 0;
                return true;
            }
        }
        false
    }
}

fn to_upper(ch: u8) -> u8 {
    if (b'a'..b'z').contains(&ch) {
        ch - b'a' + b'A'
    } else {
        ch
    }
}

pub struct AutoFileTransfer {
    zmodem_dl: PatternRecognizer,
    zmodem_ul: PatternRecognizer,
}

impl AutoFileTransfer {

    pub fn new() -> Self {
        Self {
            zmodem_dl: PatternRecognizer::from(b"**\x18B00000000000000", true),
            zmodem_ul: PatternRecognizer::from(b"**\x18B0100000023be50", true),
        }
    }

    pub fn reset(&mut self) {
        self.zmodem_dl.reset();
        self.zmodem_ul.reset();
    }

    pub fn try_transfer(&mut self, ch: u8) -> Option<(ProtocolType, bool)> {
        if self.zmodem_dl.push_ch(ch) {
            return Some((ProtocolType::ZModem, true));
        }
        if self.zmodem_ul.push_ch(ch) {
            return Some((ProtocolType::ZModem, false));
        }
        None
    }
}