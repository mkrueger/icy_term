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
pub static TXT: &[u8; 762] = b"\x1B[MFT225O3L8GL8GL8GL2E-P8L8FL8FL8FMLL2DL2DMNP8\x0E\x1B[1A\n\x1B[MFO3L8GL8GL8GL8E-L8A-L8A-L8A-L8GO4L8E-L8E-L8E-MLL2C\x0E\x1B[1A\n\x1B[MFL8CMNO3L8GL8GL8GL8DL8A-L8A-L8A-L8GO4L8FL8FL8FMLL2DL2DMN\x0E\x1B[1A\n\x1B[MFO4L8GL8GL8FL8E-O3L8E-L8E-L8FL8GO4L8GL8GL8FL8E-O3L8E-L8E-\x0E\x1B[1A\n\x1B[MFL8FL8GO4L8GL8GL8FL8E-P4L8CP4L1GO3L8A-L8A-L8A-MLL2FL2FMN\x0E\x1B[1A\n\x1B[MFP8O3L8A-L8A-L8A-L8FL8DL8DL8DO2L8BL8A-L8A-L8A-L8GO1L8GL8\x0E\x1B[1A\n\x1B[MFGL8GL8CO3L8A-L8A-L8A-L8FL8DL8DL8DO2L8B-L8A-L8A-L8A-L8GO1\x0E\x1B[1A\n\x1B[MFL8GL8GL8GL8CO3L8GO4L8CL8CL2CO3L8BL8BL8BO4L8DL2DL8DL8DL8D\x0E\x1B[1A\n\x1B[MFL8E-L8E-L8DL8DL8FL8FL8EL8E-L8GT50O4L8GL8FL8FL8A-L8A-L8G\x0E\x1B[1A\n\x1B[MFL8GL8B-L8B-L8A-L8A-L8CL8CL8BL8B-L8DL8CL8E-L8E-L8E-L8CL8\x0E\x1B[1A\n\x1B[MFGL8GL8GL8E-L8CO3L8GL8GL8E-L8CL8CL8CO2L8B-O4L8FL8E-L8E-L8\x0E\x1B[1A\n\x1B[MFBL8GL8FL8FL8DO3L8BL8GL8FL8DO2L8BO3L8CL8CL8CO4L8E-L8E-\x0E\x1B[1A";

