use std::cmp::min;

use crate::sound::play_music;

use super::main_window::{MainWindow, MainWindowMode};

pub unsafe fn run_sim(window: &mut MainWindow) {
    let upper = min(CUR_OFFSET + 2048, TXT.len());
    for offset in CUR_OFFSET..upper {
        
        match window.buffer_view.buffer_parser.print_char(
            &mut window.buffer_view.buf,
            &mut window.buffer_view.caret,
            char::from_u32_unchecked(TXT[offset] as u32),
        ) {
            Ok(act) => match act {
                icy_engine::CallbackAction::PlayMusic(m) =>play_music(m),
                _ => {}
            },
            Err(err) =>  {eprintln!("{}", err)} 
        }
        window.buffer_view.update_sixels();
    }
    CUR_OFFSET = upper;
    window.mode = MainWindowMode::ShowTerminal;
}

static mut CUR_OFFSET: usize = 0;
pub static TXT: &[u8; 762] = b"";

