use std::cmp::min;

use super::main_window::{MainWindow, MainWindowMode};

pub unsafe fn run_sim(window: &mut MainWindow) {
    let upper = min(CUR_OFFSET + 2048, TXT.len());
    for offset in CUR_OFFSET..upper {
        if let Err(err) = window.buffer_view.buffer_parser.print_char(
            &mut window.buffer_view.buf,
            &mut window.buffer_view.caret,
            char::from_u32_unchecked(TXT[offset] as u32),
        ) {
            eprintln!("{}", err);
        }
        window.buffer_view.update_sixels();
    }
    CUR_OFFSET = upper;
    window.mode = MainWindowMode::ShowTerminal;
}

static mut CUR_OFFSET: usize = 0;
static TXT: &[u8; 15342148] = b"";
