pub mod message;
use iced::{Length, widget::text};
pub use message::*;

pub mod main_window;
pub use main_window::*;

pub mod buffer_view;
pub use buffer_view::*;

pub mod hover_list;
pub use hover_list::*;

pub mod screen_modes;
pub use screen_modes::*;

pub mod phonebook;
pub use phonebook::*;

pub mod edit_bbs;
pub use edit_bbs::*;

pub mod protocol_selector;
pub use protocol_selector::*;

pub mod file_transfer;
pub use file_transfer::*;

pub mod selection;
pub use selection::*;

pub mod keymaps;
pub use keymaps::*;

// pub mod simulate;

pub fn create_icon_button(icon: &'static str) -> iced::widget::Button<'_, Message> {
    let icon_size = 24;

    let mut t = text(icon).font(iced_aw::ICON_FONT).size(icon_size);

    iced::widget::button(t)
        .padding(4)
        .on_press(Message::InitiateFileTransfer(false))
}
