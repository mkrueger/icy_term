use std::{io, cmp::{max, min}};
use super::{Buffer, Caret, Position, DosChar};
mod ascii_parser;
pub use ascii_parser::*;
mod ansi_parser;
pub use ansi_parser::*;
mod petscii_parser;
pub use petscii_parser::*;

pub trait BufferParser {

    fn from_unicode(&self, ch: char) -> u8;

    /// Prints a character to the buffer. Gives back an optional string returned to the sender (in case for terminals).
    fn print_char(&mut self, buffer: &mut Buffer, caret: &mut Caret, c: u8) -> io::Result<Option<String>>;
}


fn fill_line(buf: &mut Buffer, line:i32, from: i32, to: i32) {
    for x in from..=to {
        let p = Position::from(x, line);
        if buf.get_char(p).is_none() {
            buf.set_char( p, Some(DosChar::new()));
        }
    }
}

impl Caret {
    /// (line feed, LF, \n, ^J), moves the print head down one line, or to the left edge and down. Used as the end of line marker in most UNIX systems and variants.
    pub fn lf(&mut self, _buf: &mut Buffer) {
        self.pos.x = 0;
        self.pos.y += 1;
    }
    
    /// (form feed, FF, \f, ^L), to cause a printer to eject paper to the top of the next page, or a video terminal to clear the screen.
    pub fn ff(&mut self, _buf: &mut Buffer) {
        self.pos.x = 0;
        self.pos.y = 1;
        self.attr = super::TextAttribute::DEFAULT;
    }

    /// (carriage return, CR, \r, ^M), moves the printing position to the start of the line.
    pub fn cr(&mut self, _buf: &mut Buffer) {
        self.pos.x = 0;
    }

    pub fn eol(&mut self, buf: &mut Buffer) {
        self.pos.x = buf.width as i32 - 1;
    }

    pub fn home(&mut self, _buf: &mut Buffer) {
        self.pos = Position::new();
    }

    /// (backspace, BS, \b, ^H), may overprint the previous character
    pub fn bs(&mut self, buf: &mut Buffer) {
        self.pos.x = max(0, self.pos.x - 1);
        buf.set_char(self.pos, Some(DosChar::default()));
    }
    
    pub fn left(&mut self, buf: &mut Buffer, num: i32) {
        let old_x = self.pos.x;
        self.pos.x = max(0, self.pos.x.saturating_sub(num));
        fill_line(buf, self.pos.y, self.pos.x, old_x);
    }

    pub fn right(&mut self, buf: &mut Buffer, num: i32) {
        let old_x = self.pos.x;
        self.pos.x = min(buf.width as i32 - 1, self.pos.x + num);
        fill_line(buf, self.pos.y, old_x, self.pos.x);
    }

    pub fn up(&mut self, _buf: &mut Buffer, num: i32) {
        self.pos.y = max(0, self.pos.y.saturating_sub(num));
    }

    pub fn down(&mut self, _buf: &mut Buffer, num: i32) {
        self.pos.y = self.pos.y + num;
    }
}

impl Buffer {

    fn print_value(&mut self, caret: &mut Caret, ch: u16)
    {
        let ch = DosChar::from(ch, caret.attr);
        self.print_char(caret, ch);
    }

    fn print_char(&mut self, caret: &mut Caret, ch: DosChar)
    {
        self.set_char(caret.pos, Some(ch));
        caret.pos.x = caret.pos.x + 1;
        if caret.pos.x >= self.width as i32 {
            caret.pos.x = 0;
            caret.pos.y = caret.pos.y + 1;
        }
    }

    fn clear_screen(&mut self, caret: &mut Caret)
    {
        caret.pos = Position::new();
        self.clear();
    }

    fn clear_buffer_down(&mut self, y: i32) {
        for y in y..self.height as i32 {
            for x in 0..self.width as i32 {
                self.set_char(Position::from(x, y), Some(DosChar::new()));
            }
        }
    }

    fn clear_buffer_up(&mut self, y: i32) {
        for y in 0..y {
            for x in 0..self.width as i32 {
                self.set_char(Position::from(x, y), Some(DosChar::new()));
            }
        }
    }

    fn clear_line(&mut self, y: i32) {
        for x in 0..self.width as i32 {
            self.set_char(Position::from(x, y), Some(DosChar::new()));
        }
    }

    fn clear_line_end(&mut self, pos: &Position) {
        for x in pos.x..self.width as i32 {
            self.set_char(Position::from(x, pos.y), Some(DosChar::new()));
        }
    }

    fn clear_line_start(&mut self, pos: &Position) {
        for x in 0..pos.x {
            self.set_char(Position::from(x, pos.y), Some(DosChar::new()));
        }
    }
}