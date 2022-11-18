use super::main_window::MainWindow;
use super::screen_modes::*;
use super::Message;
use crate::address::{ Terminal};
use iced::widget::{ column, pick_list, row, text, text_input};
use iced::{alignment, Element, Length};

pub fn view_edit_bbs<'a>(main_window: &MainWindow) -> Element<'a, Message> {
    let text_width = 140;
    let padding = 10;

    if main_window.selected_address < 0 {
        return text("No selection").into();
    }
    let adr = &main_window.addresses[main_window.selected_address as usize];

    column![
    /*     row![
            button("Cancel").on_press(Message::Back),
            button("Save").on_press(Message::EditBbsSaveChanges(i)),
            button("Delete").on_press(Message::EditBbsDeleteEntry(i)),
        ]
        .padding(padding)
        .spacing(8),*/
        row![
            text("System name")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input("", &adr.system_name, Message::EditBbsSystemNameChanged)
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("Address")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input("", &adr.address, Message::EditBbsAddressChanged)
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("User")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input("", &adr.user_name, Message::EditBbsUserNameChanged)
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("Password")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input("", &adr.password, Message::EditBbsPasswordChanged)
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("Comment")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input("", &adr.comment, Message::EditBbsCommentChanged)
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("Terminal type")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            pick_list(
                &Terminal::ALL[..],
                Some(adr.terminal_type),
                Message::EditBbsTerminalTypeSelected
            )
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("Connection type")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            pick_list(
                &crate::address::ConnectionType::ALL[..],
                Some(adr.connection_type),
                Message::EditBbsConnectionType
            )
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("Screen Mode")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            pick_list(
                &DEFAULT_MODES[..],
                adr.screen_mode,
                Message::EditBbsScreenModeSelected
            )
        ]
        .padding(padding)
        .spacing(8),
        row![
            text("Autologin String")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input("", &adr.auto_login, Message::EditBbsAutoLoginChanged)
        ]
        .padding(padding)
        .spacing(8),
    ]
    .spacing(8)
    .into()
}
