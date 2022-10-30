use std::{io, cmp::{max, min}};

use crate::{model::{Position, Buffer, TextAttribute, Caret}};

use super::{BufferParser, AsciiParser};

pub struct AnsiParser {
    ascii_parser: AsciiParser,
    got_escape: bool,
    ans_code: bool,
    saved_pos: Position,
    parsed_numbers: Vec<i32>,
    current_sequence: String
}

const ANSI_CSI: u8 = b'[';
const ANSI_ESC: u8 = 27;

const COLOR_OFFSETS : [u8; 8] = [ 0, 4, 2, 6, 1, 5, 3, 7 ];

impl AnsiParser {
    pub fn new() -> Self {
        AnsiParser {
            ascii_parser: AsciiParser::new(),
            ans_code: false,
            got_escape: false,
            saved_pos: Position::new(),
            parsed_numbers: Vec::new(),
            current_sequence: String::new()
        }
    }

    fn start_sequence(&mut self, ch: u8) -> io::Result<Option<String>> {
        self.got_escape = false;
        if ch == ANSI_CSI {
            self.current_sequence.push(char::from_u32(ch as u32).unwrap());
            self.ans_code = true;
            self.parsed_numbers.clear();
        }
        return Ok(None);
    }
}

impl BufferParser for AnsiParser {
    fn from_unicode(&self, ch: char) -> u8
    {
        self.ascii_parser.from_unicode(ch)
    }

    fn print_char(&mut self, buf: &mut Buffer, caret: &mut Caret, ch: u8) -> io::Result<Option<String>>
    {
        if self.got_escape {
            return self.start_sequence(ch);
        }
    
        if self.ans_code {
            self.current_sequence.push(char::from_u32(ch as u32).unwrap());
            match ch {
                b'm' => { // Select Graphic Rendition 
                    for n in &self.parsed_numbers {
                        match n {
                            0 => caret.attr = TextAttribute::DEFAULT, // Reset or normal 
                            1 => caret.attr.set_foreground_bold(true),    // Bold or increased intensity 
                            5 => if buf.buffer_type.use_ice_colors() { 
                                caret.attr.set_background_bold(true);
                            }  else  {
                                caret.attr.set_blink(true);  // Slow blink 
                            }
    
                            // set foreaground color
                            30..=37 => caret.attr.set_foreground_without_bold(COLOR_OFFSETS[*n as usize - 30]),
                            // set background color
                            40..=47 => caret.attr.set_background_without_bold(COLOR_OFFSETS[*n as usize - 40]),
                            _ => { 
                                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Unsupported ANSI graphic code {} in seq {}", n, self.current_sequence)));
                            }
                        }
                    }
                    self.ans_code = false;
                    return Ok(None);
                }
                b'H' | b'f' => { // Cursor Position + Horizontal Vertical Position ('f')
                    if !self.parsed_numbers.is_empty() {
                        if self.parsed_numbers[0] > 0 { 
                            caret.pos.y =  max(0, self.parsed_numbers[0] - 1);
                        }
                        if self.parsed_numbers.len() > 1 {
                            if self.parsed_numbers[1] > 0 {
                                caret.pos.x =  max(0, self.parsed_numbers[1] - 1);
                            }
                        } else {
                            caret.pos.x = 0;
                        }
                    }
                    self.ans_code = false;
                    return Ok(None);
                }
                b'C' => { // Cursor Forward 
                    if self.parsed_numbers.is_empty() {
                        caret.right(buf, 1);
                    } else {
                        caret.right(buf, self.parsed_numbers[0]);
                    }
                    self.ans_code = false;
                    return Ok(None);
                }
                b'D' => { // Cursor Back 
                    if self.parsed_numbers.is_empty() {
                        caret.left(buf, 1);
                    } else {
                        caret.left(buf, self.parsed_numbers[0]);
                    }
                    self.ans_code = false;
                    return Ok(None);
                }
                b'A' => { // Cursor Up 
                    if self.parsed_numbers.is_empty() {
                        caret.up(buf, 1);
                    } else {
                        caret.up(buf, self.parsed_numbers[0]);
                    }
                    caret.pos.y = max(0, caret.pos.y);
                    self.ans_code = false;
                    return Ok(None);
                }
                b'B' => { // Cursor Down 
                    if self.parsed_numbers.is_empty() {
                        caret.down(buf, 1);
                    } else {
                        caret.down(buf, self.parsed_numbers[0]);
                    }
                    self.ans_code = false;
                    return Ok(None);
                }
                b's' => { // Save Current Cursor Position
                    self.saved_pos = caret.pos;
                    self.ans_code = false;
                    return Ok(None);
                }
                b'u' => { // Restore Saved Cursor Position 
                    caret.pos = self.saved_pos;
                    self.ans_code = false;
                    return Ok(None);
                }
                b'J' => { // Erase in Display 
                    self.ans_code = false;
                    if self.parsed_numbers.is_empty() {
                        caret.pos = Position::new();
                    } else {
                        match self.parsed_numbers.get(0).unwrap() {
                            0 => {
                                buf.clear_buffer_down(caret.pos.y);
                            }
                            1 => {
                                buf.clear_buffer_up(caret.pos.y);
                            }
                            2 |  // clear entire screen
                            3 
                            => {
                                buf.clear_screen(caret);
                            } 
                            _ => {
                                buf.clear_buffer_down(caret.pos.y);
                                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown ANSI J sequence {} in {}", self.parsed_numbers[0], self.current_sequence)));
                            }
                        }
                    }
                    return Ok(None);
                }
                b'n' => { // Device Status Report 
                    self.ans_code = false;
                    if self.parsed_numbers.is_empty() {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("empty number")));
                    }
                    if self.parsed_numbers.len() != 1 {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("too many 'n' params in ANSI escape sequence: {}", self.parsed_numbers.len())));
                    }
                    match self.parsed_numbers[0] {
                        5 => { // Device status report
                            return Ok(Some("\x1b[0n".to_string()));
                        },
                        6 => { // Get cursor position
                            let s = format!("\x1b[{};{}R", min(buf.height as i32, caret.pos.y + 1), min(buf.width as i32, caret.pos.x + 1));
                            return Ok(Some(s));
                        },
                        _ => {
                            return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown ANSI n sequence {}", self.parsed_numbers[0])));
                        }
                    }
                }
                b'K' => { // erase text
                    if self.parsed_numbers.len() > 0 {
                        match self.parsed_numbers[0] {
                            0 => { 
                                buf.clear_line_end(&caret.pos);
                            },
                            1 => {
                                buf.clear_line_start(&caret.pos);
                            },
                            2 => {
                                buf.clear_line(caret.pos.y);
                            },
                            _ => {
                                return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown ANSI K sequence {}", self.parsed_numbers[0])));
                            }
                        }
                    } else {
                        buf.clear_line_end(&caret.pos);
                    }
                    self.ans_code = false;
                    return Ok(None);
                }
                _ => {
                    if (0x40..=0x7E).contains(&ch) {
                        // unknown control sequence, terminate reading
                        self.ans_code = false;
                        self.got_escape = false;
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown control sequence {}/char:{:?} in {}", ch, char::from_u32(ch as u32), self.current_sequence)));
                    }
    
                    if (b'0'..=b'9').contains(&ch) {
                        if self.parsed_numbers.is_empty() {
                            self.parsed_numbers.push(0);
                        }
                        let d = self.parsed_numbers.pop().unwrap();
                        self.parsed_numbers.push(d * 10 + (ch - b'0') as i32);
                    } else if ch == b';' {
                        self.parsed_numbers.push(0);
                        return Ok(None);
                    } else {
                        self.ans_code = false;
                        self.got_escape = false;
                        // error in control sequence, terminate reading
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("error in ANSI control sequence: {}, {}!", self.current_sequence, ch)));
                    }
                    return Ok(None);
                }
            }
        }
    
        if ch == ANSI_ESC {
            self.current_sequence.clear();
            self.current_sequence.push_str("<ESC>");
            self.ans_code = false;
            self.got_escape = true;
            return Ok(None)
        } 
        
        self.ascii_parser.print_char(buf, caret, ch) 
    }
}

