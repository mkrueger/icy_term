use std::{io};

use crate::model::{Caret, DosChar, Position};

use super::{Buffer, BufferParser};

#[allow(clippy::struct_excessive_bools)]
pub struct PETSCIIParser {
    underline_mode: bool,
    reverse_mode: bool,
    got_esc: bool,
    shift_mode: bool
}

impl PETSCIIParser {
    pub fn new() -> Self {
        PETSCIIParser {
            shift_mode: false,
            underline_mode: false,
            reverse_mode: false,
            got_esc: false
        }
    }

    pub fn handle_reverse_mode(&self, ch : u8) -> u8 {
        if self.reverse_mode {
            ch + 0x80
        } else {
            ch
        }
    }

    pub fn handle_c128_escapes(&mut self, buf: &mut Buffer, caret: &mut Caret, ch: u8) -> io::Result<Option<String>> {
        self.got_esc = false;
            
        match ch {
            b'O' => {}, // Cancel quote and insert mode
            b'Q' => { buf.clear_line_end(&caret.pos); }, // Erase to end of current line
            b'P' => { buf.clear_line_start(&caret.pos); }, // Cancel quote and insert mode
            b'@' => { buf.clear_buffer_down(caret.pos.y); }, // Erase to end of screen
            
            b'J' => { caret.cr(buf); }, // Move to start of current line
            b'K' => { caret.eol(buf); }, // Move to end of current line
            
            b'A' => { println!("auto insert mode unsupported."); }, // Enable auto-insert mode
            b'C' => { println!("auto insert mode unsupported."); }, // Disable auto-insert mode

            b'D' => { println!("Delete current line unsupported."); }, // Delete current line
            b'I' => { println!("Insert line unsupported."); }, // Insert line

            b'Y' => { println!("Set default tab stops (8 spaces) unsupported."); }, // Set default tab stops (8 spaces)
            b'Z' => { println!("Clear all tab stops unsupported."); }, // Clear all tab stops

            b'L' => { println!("Enable scrolling unsupported."); }, // Enable scrolling
            b'M' => { println!("Disable scrolling unsupported."); }, // Disable scrolling
    
            b'V' => { println!("Scroll up unsupported."); }, // Scroll up
            b'W' => { println!("Scroll down unsupported."); }, // Scroll down

            b'G' => { println!("Enable bell unsupported."); }, // Enable bell (by CTRL G)
            b'H' => { println!("Disable bell unsupported."); }, // Disable bell

            b'E' => { println!("Set cursor to non-flashing mode unsupported."); }, // Set cursor to non-flashing mode
            b'F' => { println!("Set cursor to flashing mode unsupported."); }, // Set cursor to flashing mode

            b'B' => { println!("Set bottom of screen window at cursor position unsupported."); }, // Set bottom of screen window at cursor position
            b'T' => { println!("Set top of screen window at cursor position unsupported."); }, // Set top of screen window at cursor position

            b'X' => { println!("Swap 40/80 column display output device unsupported."); }, // Swap 40/80 column display output device
            
            b'U' => { println!("Change to underlined cursor unsupported."); }, // Change to underlined cursor
            b'S' => { println!("Change to block cursor unsupported."); }, // Change to block cursor

            b'R' => { println!("Set screen to reverse video unsupported."); }, // Set screen to reverse video
            b'N' => { println!("Set screen to normal (non reverse video) state unsupported."); }, // Set screen to normal (non reverse video) state

            _=> { println!("Unknown C128 escape code: 0x{:02X}/{} ", ch, char::from_u32(ch as u32).unwrap())}
        }
        return Ok(None);
    }

    pub fn update_shift_mode(&self, buf: &mut Buffer) 
    {
        println!("update shift mode {}", self.shift_mode);
        for y in 0..buf.height {
            for x in 0..buf.width {
                if let Some(ch) = &mut buf.get_char(Position::from(x as i32, y as i32)) {
                    ch.ext_font = self.shift_mode;
                    buf.set_char(Position::from(x as i32, y as i32), Some(*ch));
                }
            }
        }

    }
}


const BLACK:u8 = 0x00;
const WHITE:u8 = 0x01;
const RED:u8 = 0x02;
const CYAN:u8 = 0x03;
const PURPLE:u8 = 0x04;
const GREEN:u8 = 0x05;
const BLUE:u8 = 0x06;
const YELLOW:u8 = 0x07;
const ORANGE:u8 = 0x08;
const BROWN:u8 = 0x09;
const PINK:u8 = 0x0a;
const GREY1:u8 = 0x0b;
const GREY2:u8 = 0x0c;
const LIGHT_GREEN:u8 = 0x0d;
const LIGHT_BLUE:u8 = 0x0e;
const GREY3:u8 = 0x0f;

impl BufferParser for PETSCIIParser {
    fn from_unicode(&self, ch: char) -> u8
    {
        let ch = ch as u8;
        if let Some(tch) = UNICODE_TO_PETSCII.get(&ch) {
            *tch
        } else {
            ch
        }
    }

    fn print_char(&mut self, buf: &mut Buffer, caret: &mut Caret, ch: u8) -> io::Result<Option<String>> {

        if self.got_esc {
            return self.handle_c128_escapes(buf, caret, ch);
        }

        match ch {
            0x02 => self.underline_mode = true,   // C128
            0x05 => caret.set_foreground(WHITE),
            0x0A => caret.cr(buf),
            0x0D | 0x8D => { 
                caret.lf(buf);
                self.reverse_mode = false;        
            }
            0x0E | 0x8E =>  { self.shift_mode = !self.shift_mode; self.update_shift_mode(buf); },
            0x11 => caret.down(buf, 1),
            0x12 => self.reverse_mode = true,
            0x13 => caret.home(buf),
            0x14 => caret.bs(buf),
            0x1B => self.got_esc = true,
            0x1C => caret.set_foreground(RED),
            0x1D => caret.right(buf, 1),
            0x1E => caret.set_foreground(GREEN),
            0x1F => caret.set_foreground(BLUE),
            0x81 => caret.set_foreground(ORANGE),
            0x90 => caret.set_foreground(BLACK),
            0x91 => caret.up(buf, 1),
            0x92 => self.reverse_mode = false,
            0x93 => { buf.clear_screen(caret); self.shift_mode = false; } ,
            0x95 => caret.set_foreground(BROWN),
            0x96 => caret.set_foreground(PINK),
            0x97 => caret.set_foreground(GREY1),
            0x98 => caret.set_foreground(GREY2), 
            0x99 => caret.set_foreground(LIGHT_GREEN),
            0x9A => caret.set_foreground(LIGHT_BLUE),
            0x9B => caret.set_foreground(GREY3),
            0x9C => caret.set_foreground(PURPLE),
            0x9D => caret.left(buf, 1),
            0x9E => caret.set_foreground(YELLOW),
            0x9F => caret.set_foreground(CYAN),
            0xFF => buf.print_value(caret, 94), // PI character
            _ => {
                let tch = match ch {
                    0x20..=0x3F => {
                        ch
                    }
                    0x40..=0x5F => {
                        ch - 0x40
                    }
                    0x60..=0x7F => {
                        ch - 0x20
                    }
                    0xA0..=0xBF => {
                        ch - 0x40
                    }
                    0xC0..=0xFE => {
                        ch - 0x80
                    }
                    _ => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, format!("unknown control code 0x{:X}", ch)));
                    }
                };
                let mut ch = DosChar::from(self.handle_reverse_mode(tch) as u16, caret.attr);
                ch.ext_font = self.shift_mode;
                buf.print_char(caret, ch);
            }
        }
        Ok(None)
    }
}

lazy_static::lazy_static!{
    static ref UNICODE_TO_PETSCII: std::collections::HashMap<u8,u8> = vec![
        (0x41, 0x61),
        (0x42, 0x62),
        (0x43, 0x63),
        (0x44, 0x64),
        (0x45, 0x65),
        (0x46, 0x66),
        (0x47, 0x67),
        (0x48, 0x68),
        (0x49, 0x69),
        (0x4A, 0x6A),
        (0x4B, 0x6B),
        (0x4C, 0x6C),
        (0x4D, 0x6D),
        (0x4E, 0x6E),
        (0x4F, 0x6F),
        (0x50, 0x70),
        (0x51, 0x71),
        (0x52, 0x72),
        (0x53, 0x73),
        (0x54, 0x74),
        (0x55, 0x75),
        (0x56, 0x76),
        (0x57, 0x77),
        (0x58, 0x78),
        (0x59, 0x79),
        (0x5A, 0x7A),
        (0x5C, 0x9C),
        (0x5E, 0x18),
        (0x5F, 0x1B),
        (0x60, 0xC4),
        (0x61, 0x41),
        (0x62, 0x42),
        (0x63, 0x43),
        (0x64, 0x44),
        (0x65, 0x45),
        (0x66, 0x46),
        (0x67, 0x47),
        (0x68, 0x48),
        (0x69, 0x49),
        (0x6A, 0x4A),
        (0x6B, 0x4B),
        (0x6C, 0x4C),
        (0x6D, 0x4D),
        (0x6E, 0x4E),
        (0x6F, 0x4F),
        (0x70, 0x50),
        (0x71, 0x51),
        (0x72, 0x52),
        (0x73, 0x53),
        (0x74, 0x54),
        (0x75, 0x55),
        (0x76, 0x56),
        (0x77, 0x57),
        (0x78, 0x58),
        (0x79, 0x59),
        (0x7A, 0x5A),
        (0x7B, 0xC5),
        (0x7C, 0xB5),
        (0x7D, 0xB3),
        (0x7E, 0xB2),
        (0x7F, 0xB0),
        (0xA0, 0xFF),
        (0xA1, 0xDD),
        (0xA2, 0xDC),
        (0xA3, 0x5E),
        (0xA4, 0x5F),
        (0xA5, 0x7B),
        (0xA6, 0xB1),
        (0xA7, 0x7D),
        (0xA8, 0xD2),
        (0xA9, 0x1F),
        (0xAA, 0xF5),
        (0xAB, 0xC3),
        (0xAC, 0xC9),
        (0xAD, 0xC0),
        (0xAE, 0xBF),
        (0xAF, 0xCD),
        (0xB0, 0xDA),
        (0xB1, 0xC1),
        (0xB2, 0xC2),
        (0xB3, 0xB4),
        (0xB4, 0xF4),
        (0xB5, 0xB9),
        (0xB6, 0xDE),
        (0xB7, 0xA9),
        (0xB8, 0xDF),
        (0xB9, 0x16),
        (0xBA, 0xFB),
        (0xBC, 0xC8),
        (0xBD, 0xD9),
        (0xBE, 0xBC),
        (0xBF, 0xCE),
    ].into_iter().collect();
}