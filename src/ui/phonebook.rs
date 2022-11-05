use iced::widget::{ column, row, button, text, horizontal_rule, text_input, horizontal_space, vertical_space, Text};
use iced::{
    Element, Length, Alignment, theme, alignment, Font, Color
};
use super::main_window::{ MainWindow};
use lazy_static::lazy_static;
use super::Message;

lazy_static! {
    pub static ref INPUT_ID: text_input::Id = text_input::Id::unique();
}
const NAME_LEN: u16 = 350;
const ADDRESS_LEN: u16 = 250;

pub fn view_phonebook<'a>(main_window: &MainWindow) -> Element<'a, Message> {

    let list_header = column![
        row![
            horizontal_space(Length::Units(20)),
            button("Quick Connect")
                .on_press(Message::CallBBS(0))
                .padding(10),
            horizontal_space(Length::Units(10)),
            text_input(
                "",
                &main_window.addresses[0].address,
                Message::QuickConnectChanged
            )
            .id(INPUT_ID.clone())
            .size(40),
            horizontal_space(Length::Units(20)),
        ].align_items(Alignment::Center),
        vertical_space(Length::Units(10)),

        row![
            horizontal_space(Length::Units(118 + 36)),
            text("Name").size(26).width(Length::Units(NAME_LEN)),
            text("Comment").size(26).width(Length::Fill),
            text("Address").size(26).width(Length::Units(ADDRESS_LEN)),
        ].align_items(Alignment::Center),
        row![
            horizontal_rule(5)
        ],
    ].spacing(8)
    ;

    let list: Element<'a, Message> = if main_window.addresses.len() > 0 {
        column(
            main_window.addresses
            .iter()
            .skip(1)
            .enumerate()
            .map(|(i, adr)| {
                row![
                    horizontal_space(Length::Units(20)),
                    button("Connect")
                        .on_press(Message::CallBBS(i + 1)),
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
                .align_items(Alignment::Center).into()
            }).collect()
        )
        .spacing(10)
        .into()
    } else {
        text("No BBSes yet…").into()
    };
    column![
        row![
            button("Back")
                .on_press(Message::Back),
            button("New")
                .on_press(Message::EditBBS(0)),
            button("Get More")
            .on_press(Message::OpenURL("https://www.telnetbbsguide.com/".to_string())),
            ].padding(4)
        .spacing(8),

        text("Connect to")
        .width(Length::Fill)
        .size(50)
        .style(Color::from([0.5, 0.5, 0.5]))
        .horizontal_alignment(alignment::Horizontal::Center),

        list_header,
        
        iced::widget::scrollable(iced::widget::container(list).width(Length::Fill).center_x())
    ].spacing(8).into()
}

const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../fonts/icons.ttf"),
};

fn edit_icon() -> Text<'static> {
    icon('\u{F303}')
}

fn icon(unicode: char) -> Text<'static> {
    text(unicode.to_string())
        .font(ICONS)
        .width(Length::Units(20))
        .horizontal_alignment(alignment::Horizontal::Center)
        .size(20)
}
