use std::{io, cmp::{max, min}};

use crate::com::Com;

use super::{Position, Buffer, TextAttribute, DosChar};

#[allow(clippy::struct_excessive_bools)]
pub struct ParseStates {
    pub screen_width: u16,
    
    // ANSI
    pub ans_esc: bool,
    pub ans_code: bool,
    pub saved_pos: Position,
    pub ans_numbers: Vec<i32>,
    pub ans_seq: String

    // Avatar
   /* pub avt_state: AvtReadState,
    pub avatar_state: i32,
    pub avt_repeat_char: u8,
    pub avt_repeat_count: i32*/
}
impl ParseStates {
    pub fn new() -> Self {
        ParseStates {
            screen_width: 80,
            ans_code: false,
            ans_esc: false,
            saved_pos: Position::new(),
            ans_numbers: Vec::new(),
            ans_seq: String::new()
        }
    }
}

const ANSI_CSI: u8 = b'[';
const ANSI_ESC: u8 = 27;

const COLOR_OFFSETS : [u8; 8] = [ 0, 4, 2, 6, 1, 5, 3, 7 ];

pub fn display_ans<T: Com>(buf: &mut Buffer, caret: &mut Position, attr: &mut TextAttribute, data: &mut ParseStates, telnet: &mut T, ch: u8) -> io::Result<Option<u8>> {
    if data.ans_esc {
        data.ans_seq.push(char::from_u32(ch as u32).unwrap());

        if ch == ANSI_CSI {
            data.ans_esc = false;
            data.ans_code = true;
            data.ans_numbers.clear();
            return Ok(None);
        }
        // ignore all other ANSI escape codes
        data.ans_esc = false;
        return Ok(None);
    }

    if data.ans_code {
        data.ans_seq.push(char::from_u32(ch as u32).unwrap());
        match ch {
            b'm' => { // Select Graphic Rendition 
                for n in &data.ans_numbers {
                    match n {
                        0 => *attr = TextAttribute::DEFAULT, // Reset or normal 
                        1 => attr.set_foreground_bold(true),    // Bold or increased intensity 
                        5 => if buf.buffer_type.use_ice_colors() { 
                            attr.set_background_bold(true);
                        }  else  {
                            attr.set_blink(true);  // Slow blink 
                        }

                        // set foreaground color
                        30..=37 => attr.set_foreground_without_bold(COLOR_OFFSETS[*n as usize - 30]),
                        // set background color
                        40..=47 => attr.set_background_without_bold(COLOR_OFFSETS[*n as usize - 40]),
                        _ => { 
                            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Unsupported ANSI graphic code {} in seq {}", n, data.ans_seq)));
                        }
                    }
                }
                data.ans_code = false;
                return Ok(None);
            }
            b'H' | b'f' => { // Cursor Position + Horizontal Vertical Position ('f')
                if !data.ans_numbers.is_empty() {
                    if data.ans_numbers[0] > 0 { 
                        caret.y =  max(0, data.ans_numbers[0] - 1);
                    }
                    if data.ans_numbers.len() > 1 {
                        if data.ans_numbers[1] > 0 {
                            caret.x =  max(0, data.ans_numbers[1] - 1);
                        }
                    } else {
                        caret.x = 0;
                    }
                }
                data.ans_code = false;
                return Ok(None);
            }
            b'C' => { // Cursor Forward 
                let old_x = caret.x;
                if data.ans_numbers.is_empty() {
                    caret.x += 1;
                } else {
                    caret.x += data.ans_numbers[0];
                }
                caret.x = min(data.screen_width as i32 - 1, caret.x);
                for x in old_x..=caret.x {
                    let p =Position::from(x, caret.y);
                    if buf.get_char(p).is_none() {
                        buf.set_char( p, Some(DosChar::new()));
                    }
                }
                data.ans_code = false;
                // buf.height = max( buf.height, caret.y as u16 + 1);
                return Ok(None);
            }
            b'D' => { // Cursor Back 
                if data.ans_numbers.is_empty() {
                    caret.x = max(0, caret.x - 1);
                } else {
                    caret.x =  max(0, caret.x.saturating_sub(data.ans_numbers[0]));
                }
                caret.x = max(0, caret.x);
                data.ans_code = false;
                return Ok(None);
            }
            b'A' => { // Cursor Up 
                if data.ans_numbers.is_empty() {
                    caret.y =  max(0, caret.y - 1);
                } else {
                    caret.y = max(0, caret.y.saturating_sub(data.ans_numbers[0]));
                }
                caret.y = max(0, caret.y);
                data.ans_code = false;
                return Ok(None);
            }
            b'B' => { // Cursor Down 
                if data.ans_numbers.is_empty() {
                    caret.y += 1;
                } else {
                    caret.y += data.ans_numbers[0];
                }
                data.ans_code = false;
                return Ok(None);
            }
            b's' => { // Save Current Cursor Position
                data.saved_pos = *caret;
                data.ans_code = false;
                return Ok(None);
            }
            b'u' => { // Restore Saved Cursor Position 
                *caret = data.saved_pos;
                data.ans_code = false;
                return Ok(None);
            }
            b'J' => { // Erase in Display 
                data.ans_code = false;
                if data.ans_numbers.is_empty() {
                    *caret = Position::new();
                } else {
                    match data.ans_numbers.get(0).unwrap() {
                        0 => {
                            buf.clear_buffer_down(caret.y);
                        }
                        1 => {
                            buf.clear_buffer_up(caret.y);
                        }
                        2 |  // clear entire screen
                        3 
                        => {
                            *caret = Position::new();
                            buf.clear();
                        } 
                        _ => {
                            buf.clear_buffer_down(caret.y);
                            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown ANSI J sequence {} in {}", data.ans_numbers[0], data.ans_seq)));
                        }
                    }
                }
                return Ok(None);
            }
            b'n' => {  // Device Status Report 
                data.ans_code = false;
                if data.ans_numbers.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("empty number")));
                }
                if data.ans_numbers.len() != 1 {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("too many 'n' params in ANSI escape sequence: {}", data.ans_numbers.len())));
                }
                match data.ans_numbers[0] {
                    5 => { // Device status report
                        telnet.write(format!("\x1b[0n").as_bytes())?;
                    },
                    6 => { // Get cursor position
                        let s = format!("\x1b[{};{}R", min(buf.height as i32, caret.y + 1), min(buf.width as i32, caret.x + 1));
                        println!("send cursor position <ESC>[{};{}R", min(buf.height as i32, caret.y + 1), min(buf.width as i32, caret.x + 1));
                        telnet.write(s.as_bytes())?;
                    },
                    _ => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown ANSI n sequence {}", data.ans_numbers[0])));
                    }
                }
            }
            b'K' => { // erase text
                if data.ans_numbers.len() > 0 {
                    match data.ans_numbers[0] {
                        0 => { 
                            buf.clear_line_end(&caret);
                        },
                        1 => {
                            buf.clear_line_start(&caret);
                        },
                        2 => {
                            buf.clear_line(caret.y);
                        },
                        _ => {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown ANSI K sequence {}", data.ans_numbers[0])));
                        }
                    }
                } else {
                    buf.clear_line_end(caret);
                }
                data.ans_code = false;
                return Ok(None);
            }
            _ => {
                if (0x40..=0x7E).contains(&ch) {
                    // unknown control sequence, terminate reading
                    data.ans_code = false;
                    data.ans_esc = false;
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown control sequence {}/char:{:?} in {}", ch, char::from_u32(ch as u32), data.ans_seq)));
                }

                if (b'0'..=b'9').contains(&ch) {
                    if data.ans_numbers.is_empty() {
                        data.ans_numbers.push(0);
                    }
                    let d = data.ans_numbers.pop().unwrap();
                    data.ans_numbers.push(d * 10 + (ch - b'0') as i32);
                } else if ch == b';' {
                    data.ans_numbers.push(0);
                    return Ok(None);
                } else {
                    data.ans_code = false;
                    data.ans_esc = false;
                    // error in control sequence, terminate reading
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("error in ANSI control sequence: {}, {}!", data.ans_seq, ch)));
                }
                return Ok(None);
            }
        }
    }

    if ch == ANSI_ESC {
        data.ans_seq.clear();
        data.ans_seq.push_str("<ESC>");
        data.ans_code = false;
        data.ans_esc = true;
        return Ok(None)
    } else {
        return Ok(Some(ch))
    }
}