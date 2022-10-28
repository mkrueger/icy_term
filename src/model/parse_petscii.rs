use std::io;

use crate::com::Com;

use super::{Buffer, Position, TextAttribute};


#[allow(clippy::struct_excessive_bools)]
pub struct PETSCIIState {
    pub ext_font: bool,
    pub underline_mode: bool,
    pub reverse_mode: bool
}

impl PETSCIIState {
    pub fn new() -> Self {
        PETSCIIState {
            ext_font: false,
            underline_mode: false,
            reverse_mode: false
        }
    }
}

pub fn parse_petscii<T: Com>(buf: &mut Buffer, caret: &mut Position, attr: &mut TextAttribute, state: &mut PETSCIIState, _com: &mut T, ch: u8) -> io::Result<Option<u8>> {
    if ch >= 0x20 && ch <= 0x7F || ch >= 0xA0 {
        return Ok(Some(ch));
    }
    match ch {
        0x02 => { state.underline_mode = true; } // C128
        0x05 => { attr.set_foreground(1); } // WHITE
        0x0D => { return Ok(Some(ch)); } // RETURN
        0x0E => { state.ext_font = !state.ext_font; } // toggle up/low case
        0x11 => { caret.y += 1; } // caret down
        0x12 => { state.reverse_mode = true; }
        0x13 => { caret.x = 0; caret.y = 0;  } // home
//        0x14 => {   } // remove char left from caret
        0x1C => {  attr.set_foreground(2);  } // RED
        0x1D => {  caret.x += 1;  } // Caret right
        0x1E => {  attr.set_foreground(5);  } // GREEN
        0x1F => {  attr.set_foreground(6);  } // BLUE
        
        0x81 => {  attr.set_foreground(8);  } // ORANGE
        0x8D => {  return Ok(Some(b'\n'));  } // Carriage return
        0x90 => {  attr.set_foreground(0);  } // BLACK
        0x91 => {   caret.y -= 1; } // caret up
        0x92 => { state.reverse_mode = false; } // reverse mode off
        0x93 => { // clear screen
            *caret = Position::new();
            buf.clear();
        } 
        0x95 => {  attr.set_foreground(9);  } // BROWN
        0x96 => {  attr.set_foreground(10);  } // LIGHT RED
        0x97 => {  attr.set_foreground(11);  } // {GRY 1} 
        0x98 => {  attr.set_foreground(12);  } // {GRY 2} 
        0x99 => {  attr.set_foreground(13);  } // {L GRN} 
        0x9A => {  attr.set_foreground(14);  } // {L BLU} 
        0x9B => {  attr.set_foreground(15);  } // {GRY 3} 
        0x9C => {  attr.set_foreground(4);  } // {PUR} 
        0x9D => {  caret.x -= 1  } // Caret left
        0x9E => {  attr.set_foreground(7);  } // {YEL} 
        0x9F => {  attr.set_foreground(3);  } // {CYN} 

        _ => { println!("unknown control code 0x{:X}", ch); }
    }

    Ok(None)
}