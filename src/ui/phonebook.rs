use iced::widget::{ column, row, button, text, horizontal_rule, text_input};
use iced::{
    Element, Length, Alignment
};
use crate::com::{Com};
use super::main_window::{Message, MainWindow};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref INPUT_ID: text_input::Id = text_input::Id::unique();
}

pub fn view_phonebook<'a, T: Com>(main_window: &MainWindow<T>) -> Element<'a, Message> {
    let list: Element<'a, Message> = if main_window.addresses.len() > 0 {
        let  p = vec![
            row![
                text_input(
                    "Quick connect to",
                    &main_window.addresses[0].address,
                    Message::QuickConnectChanged
                )
                .id(INPUT_ID.clone())
                .size(30),
                button("Dial")
                        .on_press(Message::CallBBS(0))
                        .padding(10),
            ].spacing(22).align_items(Alignment::Center).into(),
            row![
                text("System").size(20).width(Length::Units(400)),
                //vertical_rule(2),
                text("Comment").size(20).width(Length::Fill),
            //  vertical_rule(2),
                text("User name").size(20).width(Length::Units(100)),
            ].spacing(30)
            .align_items(Alignment::Center).into(),
            row![
                horizontal_rule(5)
            ].into(),
            
        ];

        let p2 = main_window.addresses
            .iter()
            .skip(1)
            .enumerate()
            .map(|(i, adr)| {
                row![
                    row![
                        button("Dial")
                        .on_press(Message::CallBBS(i + 1))
                        .padding(10),
                    text(adr.system_name.to_string())].spacing(10).align_items(Alignment::Center).width(Length::Units(400)),
                    //vertical_rule(2),
                    text(adr.comment.to_string()).width(Length::Fill),
                //  vertical_rule(2),
                    text(adr.user_name.to_string()).width(Length::Units(100)),
                ]
                .spacing(30)
                .align_items(Alignment::Center).into()
            });
            
        column(
            p.into_iter().chain(p2).collect()
        )
        .spacing(10)
        .into()
    } else {
        text("No BBSes yet…").into()
    };
    let s = iced::widget::scrollable(
            iced::widget::container(list)
            .width(Length::Fill)
            .padding(40)
            .center_x(),
    );
    column![
        row![
            button("Back")
                .on_press(Message::Back),
            button("Edit")
                .on_press(Message::Edit),
        ].padding(4)
        .spacing(8),
        s
    ].spacing(8).into()
}