use iced::widget::{ column, row, button, text, text_input, pick_list};
use iced::{
    Element, alignment, Length
};
use crate::address::{Terminal, Address};
use super::Message;
use super::main_window::{ MainWindow};
use super::screen_modes::*;


pub fn view_edit_bbs<'a>(_main_window: &MainWindow, adr: &Address, i: usize) -> Element<'a, Message> {
    let text_width = 140;
    let padding = 10;
    column![
        row![
            button("Cancel")
                .on_press(Message::Back),
            button("Save")
                .on_press(Message::EditBbsSaveChanges(i)),
            button("Delete")
                .on_press(Message::EditBbsDeleteEntry(i)),
        ].padding(padding)
        .spacing(8),

        row![
            text("System name")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input(
                "",
                &adr.system_name,
                Message::EditBbsSystemNameChanged
            )
        ].padding(padding)
        .spacing(8),
        
        row![
            text("Address")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input(
                "",
                &adr.address,
                Message::EditBbsAddressChanged
            )
        ].padding(padding)
        .spacing(8),

        row![
            text("User")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input(
                "",
                &adr.user_name,
                Message::EditBbsUserNameChanged
            )
        ].padding(padding)
        .spacing(8),

        row![
            text("Password")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input(
                "",
                &adr.password,
                Message::EditBbsPasswordChanged
            )
        ].padding(padding)
        .spacing(8),

        row![
            text("Comment")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input(
                "",
                &adr.comment,
                Message::EditBbsCommentChanged
            )
        ].padding(padding)
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
        ].padding(padding)
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
        ].padding(padding)
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
        ].padding(padding)
        .spacing(8),

        row![
            text("Autologin String")
                .horizontal_alignment(alignment::Horizontal::Right)
                .width(Length::Units(text_width)),
            text_input(
                "",
                &adr.auto_login,
                Message::EditBbsAutoLoginChanged
            )
        ].padding(padding)
        .spacing(8),

    ].spacing(8).into()
}