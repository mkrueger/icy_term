use std::{io};


use super::{Buffer, Position, TextAttribute, DosChar};
#[allow(clippy::struct_excessive_bools)]
pub struct PETSCIIState {
    pub ext_font: bool,
    pub underline_mode: bool,
    pub reverse_mode: bool,
    pub got_esc: bool,
    pub shift_mode: bool,
    reset_reverse: bool,
}

impl PETSCIIState {
    pub fn new() -> Self {
        PETSCIIState {
            ext_font: false,
            shift_mode: false,
            underline_mode: false,
            reverse_mode: false,
            got_esc: false,
            reset_reverse: false
        }
    }

    pub fn handle_reverse(&self, ch : u8) -> u8 {
        if self.reverse_mode {
            ch + 0x80
        } else {
            ch
        }
    }

    pub fn set_color(&mut self, attr: &mut TextAttribute, color: u8) {
            attr.set_foreground(color); 
    }
}

pub fn parse_petscii(buf: &mut Buffer, caret: &mut Position, attr: &mut TextAttribute, state: &mut PETSCIIState, ch: u8) -> io::Result<Option<u8>> {
  
    if state.reset_reverse  {
        state.reset_reverse = false; 
    }
 
/* 
    if ch >= 0x20 && ch <= 0x7F || ch >= 0xA0 {

        let ch = if state.reverse_mode {

            if ch <= 0x7F {
                ch + 0x80
            } else { ch }
        } else { ch };
    }*/

    if state.got_esc {
        state.got_esc = false;
        
        match ch {
            b'O' => {}, // Cancel quote and insert mode
            b'Q' => { buf.clear_line_end(&caret); }, // Erase to end of current line
            b'P' => { buf.clear_line_start(&caret); }, // Cancel quote and insert mode
            b'@' => { buf.clear_buffer_down(caret.y); }, // Erase to end of screen
            
            b'J' => { caret.x = 0; }, // Move to start of current line
            b'K' => { caret.x = buf.width as i32 - 1; }, // Move to end of current line
            
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
    match ch {
        0x02 => { state.underline_mode = true; }   // C128
        0x05 => { state.set_color(attr, 1); } // WHITE
        0x0A => { caret.x = 0; }       // RETURN
        0x0D | 0x8D => { 
            caret.x = 0;
            caret.y += 1; 
            state.reverse_mode = false;        
        }      // RETURN
        0x0E => { state.ext_font = !state.ext_font; println!("Toggle up/low casing!") } // toggle up/low case
        0x11 => { caret.y += 1; }               // caret down
        0x12 => { if !state.reverse_mode {
            state.reverse_mode = true;  
        }} // reverse mode on
        0x13 => { caret.x = 0; caret.y = 0;  }  // home
        0x14 => { buf.set_char(Position { x: caret.x - 1, y: caret.y }, Some(DosChar::default()));  } // remove char left from caret
        0x1B => { state.got_esc = true; }
        0x1C => { state.set_color(attr, 2);  } // RED
        0x1D => { caret.x += 1;  }                  // Caret right
        0x1E => { state.set_color(attr, 5);  } // GREEN 
        0x1F => { state.set_color(attr, 6);  } // BLUE
        
        0x81 => { state.set_color(attr, 8);  } // ORANGE
        0x8E => { state.ext_font = !state.ext_font; println!("Toggle up/low casing 2!") } // toggle up/low case
        0x90 => { state.set_color(attr, 0);  } // BLACK
        0x91 => { caret.y -= 1; }                  // caret up
        0x92 => { if state.reverse_mode {
            state.reverse_mode = false;  

        } }    // reverse mode off
        0x93 => {                                  // clear screen
            *caret = Position::new();
            buf.clear();
        } 
        0x95 => {  state.set_color(attr, 9);  } // BROWN
        0x96 => {  state.set_color(attr, 10);  } // LIGHT RED
        0x97 => {  state.set_color(attr, 11);  } // {GRY 1} 
        0x98 => {  state.set_color(attr, 12);  } // {GRY 2} 
        0x99 => {  state.set_color(attr, 13);  } // {L GRN} 
        0x9A => {  state.set_color(attr, 14);  } // {L BLU} 
        0x9B => {  state.set_color(attr, 15);  } // {GRY 3} 
        0x9C => {  state.set_color(attr, 4);  } // {PUR} 
        0x9D => {  caret.x -= 1  }                  // Caret left
        0x9E => {  state.set_color(attr, 7);  } // {YEL} 
        0x9F => {  state.set_color(attr, 3);  } // {CYN} 

        _ => { 
            if ch >= 0x20 && ch < 0x40 {
                return Ok(Some(state.handle_reverse(ch)));
            }
            if ch >= 0x40 && ch < 0x5f {
                return Ok(Some(state.handle_reverse(ch - 0x40)));
            }
            if ch >= 0x60 && ch < 0x7f {
                return Ok(Some(state.handle_reverse(ch - 0x20)));
            }
            if ch >= 0xA0 && ch < 0xBf {
                return Ok(Some(state.handle_reverse(ch - 0x40)));
            }
            if ch >= 0xC0 && ch < 0xFE {
                return Ok(Some(state.handle_reverse(ch - 0x80)));
            }
            println!("unknown control code 0x{:X}", ch); 
        }
    }

    Ok(None)
}

/* C128 Escape codes:
*/