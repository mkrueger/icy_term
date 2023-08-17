pub mod main_window_mod;
pub use main_window_mod::*;

pub mod buffer_view;
pub use buffer_view::*;

pub mod terminal_window;
pub use terminal_window::*;

/*
pub mod hover_list;
pub use hover_list::*;
*/
pub mod screen_modes;
pub use screen_modes::*;

pub mod phonebook_mod;
pub use phonebook_mod::*;

pub mod protocol_selector;
pub use protocol_selector::*;

pub mod file_transfer;
pub use file_transfer::*;

pub mod keymaps;
pub use keymaps::*;

pub mod settings_dialog;
pub use settings_dialog::*;

pub mod options;
pub use options::*;

// pub mod simulate;

impl BufferInputMode {
    pub fn cur_map<'a>(&self) -> &'a [(u32, &[u8])] {
        match self {
            BufferInputMode::CP437 => ANSI_KEY_MAP,
            BufferInputMode::PETscii => C64_KEY_MAP,
            BufferInputMode::ATAscii => ATASCII_KEY_MAP,
            //     super::BufferInputMode::VT500 => super::VT500_KEY_MAP,
            BufferInputMode::ViewData => VIDEOTERM_KEY_MAP,
        }
    }
}
