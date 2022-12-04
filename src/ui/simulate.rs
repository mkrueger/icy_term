
use super::main_window::{MainWindow};

pub unsafe fn run_sim(window: &mut MainWindow) {
    for c in TXT.chars() {
        window.print_char(c as u8).unwrap()                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             ;
    }
}


pub static TXT: &str = "Sixel:\n";

// \x1bPq#0;2;0;0;0#1;2;100;100;0#2;2;0;100;0#1~~@@vv@@~~@@~~$#2??}}GG}}??}}??-#1!14@\x1b\\