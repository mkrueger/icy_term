use super::main_window::MainWindow;
use super::{create_icon_button, Message};
use iced::widget::{
    button, column, horizontal_rule, horizontal_space, row, text, text_input, Column, Row, Text,
};
use iced::{alignment, theme, Alignment, Color, Element, Font, Length};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref INPUT_ID: text_input::Id = text_input::Id::unique();
}
const NAME_LEN: u16 = 350;
const ADDRESS_LEN: u16 = 250;

static LOGIN_SVG: &[u8] = include_bytes!("../../resources/login.svg");

pub fn view_phonebook<'a>(main_window: &MainWindow) -> Element<'a, Message> {
    let list_header = Column::new()
        .push(
            Row::new()
                .push(horizontal_space(Length::Units(20)))
                .push(create_icon_button(LOGIN_SVG).on_press(Message::CallBBS(0)))
                .push(horizontal_space(Length::Units(10)))
                .push(
                    text_input(
                        "Quick connect to…",
                        &main_window.addresses[0].address,
                        Message::QuickConnectChanged,
                    )
                    .id(INPUT_ID.clone())
                    .padding(8)
                    .size(18),
                )
                .push(horizontal_space(Length::Units(10)))
                .align_items(Alignment::Center),
        )
        .push(
            Row::new()
                .push(horizontal_space(Length::Units(118 + 36)))
                .push(text("Name").size(26).width(Length::Units(NAME_LEN)))
                .push(text("Comment").size(26).width(Length::Fill))
                .push(text("Address").size(26).width(Length::Units(ADDRESS_LEN)))
                .align_items(Alignment::Center),
        )
        .push(horizontal_rule(5))
        .spacing(8);

    let list: Element<'a, Message> = if main_window.addresses.len() > 0 {
        column(
            main_window
                .addresses
                .iter()
                .skip(1)
                .enumerate()
                .map(|(i, adr)| {
                    row![
                        horizontal_space(Length::Units(20)),
                        create_icon_button(LOGIN_SVG)
                            .on_press(Message::CallBBS(i + 1))
                            .style(theme::Button::Text),
                        button(edit_icon())
                            .on_press(Message::EditBBS(i + 1))
                            .style(theme::Button::Text),
                        text(i.to_string())
                            .horizontal_alignment(alignment::Horizontal::Right)
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .width(Length::Units(30)),
                        horizontal_space(Length::Units(6)),
                        text(adr.system_name.to_string()).width(Length::Units(NAME_LEN)),
                        text(adr.comment.to_string()).width(Length::Fill),
                        text(adr.address.to_string()).width(Length::Units(ADDRESS_LEN)),
                    ]
                    .align_items(Alignment::Center)
                    .into()
                })
                .collect(),
        )
        .spacing(10)
        .into()
    } else {
        text("No BBSes yet…").into()
    };
    column![
        row![
            button("New").on_press(Message::EditBBS(0)),
            button("Get More").on_press(Message::OpenURL(
                "https://www.telnetbbsguide.com/".to_string()
            )),
        ]
        .padding(4)
        .spacing(8),
        list_header,
        iced::widget::scrollable(iced::widget::container(list).width(Length::Fill).center_x())
    ]
    .spacing(8)
    .into()
}

fn edit_icon() -> Text<'static> {
    icon('\u{F303}')
}

fn icon(unicode: char) -> Text<'static> {
    text(unicode.to_string())
        .width(Length::Units(20))
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}
