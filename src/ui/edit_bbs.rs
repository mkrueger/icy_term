use super::main_window::MainWindow;
use super::screen_modes::*;
use super::Message;
use crate::address::{ Terminal};
use iced::Alignment;
use iced::widget::Column;
use iced::widget::Row;
use iced::widget::button;
use iced::widget::horizontal_space;
use iced::widget::{pick_list, text, text_input};
use iced::{alignment, Element, Length};
const TEXT_WIDTH:u16 = 140;

pub fn view_edit_bbs<'a>(main_window: &MainWindow) -> Element<'a, Message> {

    if main_window.address_list.selected_item < 0 || main_window.address_list.selected_item >= main_window.addresses.len() {
        return 
        Column::new()
        .push(text("No selection"))
        .padding(20)
        .into();
    }
    let adr = &main_window.addresses[main_window.address_list.selected_item as usize];

    let mut screen_mode_row = create_row("Screen Mode")
    .push(pick_list(
        &DEFAULT_MODES[..],
        adr.screen_mode,
        Message::EditBbsScreenModeSelected
    ));
    
    if let Some(ScreenMode::DOS(_, _)) = &adr.screen_mode {
        screen_mode_row = screen_mode_row.push(text("Terminal type"))
        .push(pick_list(
                &Terminal::ALL[..],
                Some(adr.terminal_type),
                Message::EditBbsTerminalTypeSelected
            )
        );
    }

    let mut pw_row = create_row("Password")
    .push(text_input("", &adr.password, Message::EditBbsPasswordChanged));

    //if adr.password.len() == 0 
    {
        pw_row = pw_row.push(button("Generate").on_press(Message::GeneratePassword))
    }

    Column::new()
        .push(create_row("Name").push(text_input("", &adr.system_name, Message::EditBbsSystemNameChanged)))
        .push(
            create_row("Address")
            .push(text_input("", &adr.address, Message::EditBbsAddressChanged))
            .push(
                pick_list(
                    &crate::address::ConnectionType::ALL[..],
                    Some(adr.connection_type),
                    Message::EditBbsConnectionType
                ))
        )
        .push(horizontal_space(Length::Units(8)))
        .push(create_row("User").push(text_input("", &adr.user_name, Message::EditBbsUserNameChanged)))
        .push(
            pw_row
        )

        .push(horizontal_space(Length::Units(8)))
        .push(screen_mode_row)
    .push(create_row("Autologin String").push(text_input("", &adr.auto_login, Message::EditBbsAutoLoginChanged)))
    .push(create_row("Comment").push(text_input("", &adr.comment, Message::EditBbsCommentChanged)))
    .padding(10)
    .spacing(8)
    .into()
}

fn create_row<'a, Message, Renderer>(title: &'static str) -> iced_native::widget::Row<'a, Message, Renderer> 
    where 
        Renderer: 'a + iced_native::text::Renderer, Renderer::Theme: text::StyleSheet,
{
    let r = Row::new()
        .align_items(Alignment::Center)
        .spacing(10);
 
    if title.len() > 0 {
        r.push(
            text(title)
            .horizontal_alignment(alignment::Horizontal::Right)
            .width(Length::Units(TEXT_WIDTH))
        )
    } else {
        r
    }
}
