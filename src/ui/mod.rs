pub mod message;
use iced::Length;
pub use message::*;

pub mod main_window;
pub use main_window::*;

pub mod buffer_view;
pub use buffer_view::*;

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

pub fn create_icon_button(icon_data: &'static [u8]) -> iced::widget::Button<'_, Message>  {
    let icon_size = 32;

    let icon = iced::widget::svg(iced::widget::svg::Handle::from_memory(icon_data))
        .width(Length::Units(icon_size))
        .height(Length::Units(icon_size));

    iced::widget::button(icon)
        .padding(0)
        .on_press(Message::InitiateFileTransfer(false))
}
