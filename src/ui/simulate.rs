
use super::main_window::{MainWindow};

pub unsafe fn run_sim(window: &mut MainWindow) {
    for c in TXT.chars() {
        window.print_char(c as u8).unwrap();
    }
}

pub static TXT: &str = "";
