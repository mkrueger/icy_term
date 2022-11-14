use super::Message;
use crate::protocol::ProtocolType;
use iced::widget::{button, column, horizontal_space, row, text};
use iced::{Element, Length};

pub fn view_protocol_selector<'a>(download: bool) -> Element<'a, Message> {
    let button_width = 120;
    let space = 8;
    let left = 20;

    column![
        row![button("Cancel").on_press(Message::Back),]
            .padding(4)
            .spacing(8),
        text(format!(
            "Select {} protocol",
            if download { "download" } else { "upload" }
        ))
        .size(40),
        row![
            horizontal_space(Length::Units(left)),
            button("Zmodem")
                .on_press(Message::SelectProtocol(ProtocolType::ZModem, download))
                .width(Length::Units(button_width)),
            horizontal_space(Length::Units(space)),
            text("The standard protocol")
        ],
        row![
            horizontal_space(Length::Units(left)),
            button("ZedZap")
                .on_press(Message::SelectProtocol(ProtocolType::ZedZap, download))
                .width(Length::Units(button_width)),
            horizontal_space(Length::Units(space)),
            text("8k Zmodem")
        ],
        row![
            horizontal_space(Length::Units(left)),
            button("Xmodem")
                .on_press(Message::SelectProtocol(ProtocolType::XModem, download))
                .width(Length::Units(button_width)),
            horizontal_space(Length::Units(space)),
            text("Outdated protocol")
        ],
        row![
            horizontal_space(Length::Units(left)),
            button("Xmodem 1k")
                .on_press(Message::SelectProtocol(ProtocolType::XModem1k, download))
                .width(Length::Units(button_width)),
            horizontal_space(Length::Units(space)),
            text("Rarely used anymore")
        ],
        row![
            horizontal_space(Length::Units(left)),
            button("Xmodem 1k-G")
                .on_press(Message::SelectProtocol(ProtocolType::XModem1kG, download))
                .width(Length::Units(button_width)),
            horizontal_space(Length::Units(space)),
            text("Does that even exist?")
        ],
        row![
            horizontal_space(Length::Units(left)),
            button("Ymodem")
                .on_press(Message::SelectProtocol(ProtocolType::YModem, download))
                .width(Length::Units(button_width)),
            horizontal_space(Length::Units(space)),
            text("Ok but Zmodem is better")
        ],
        row![
            horizontal_space(Length::Units(left)),
            button("Ymodem-G")
                .on_press(Message::SelectProtocol(ProtocolType::YModemG, download))
                .width(Length::Units(button_width)),
            horizontal_space(Length::Units(space)),
            text("A fast Ymodem variant")
        ]
    ]
    .spacing(8)
    .into()
}
