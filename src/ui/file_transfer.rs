use std::cmp::{max};
use std::time::{Duration, SystemTime};
use iced::{
    Element, Length, Alignment, alignment, Color
};

use gabi::BytesConfig;
use iced::widget::{ column, row, progress_bar, text, button, horizontal_space, horizontal_rule};

use crate::protocol::{ TransferState };
use super::Message;

pub fn view_file_transfer<'a>(state: &TransferState, download: bool) ->Element<'a, Message> {
    if let Some(transfer_state) = if download { state.recieve_state.as_ref() } else { state.send_state.as_ref() } {
        let check = transfer_state.check_size.clone();
        let file_name = transfer_state.file_name.clone();
        let file_size  = transfer_state.file_size;
        let current_state = state.current_state.to_string();

        let bps  = transfer_state.get_bps();
        let bytes_left = transfer_state.file_size - transfer_state.bytes_transfered;
        let time_left = Duration::from_secs(bytes_left as u64 / max(1, bps));

        let bb = BytesConfig::default();

        let left_size = 100;

        let elapsed_time = SystemTime::now().duration_since(state.start_time).unwrap();
        let elapsed_time = format!("{:02}:{:02}", elapsed_time.as_secs() / 60,  elapsed_time.as_secs() % 60);

        let log = column(
            transfer_state.output_log
            .iter()
            .rev()
            .take(5)
            .rev()
            .map(|txt| {
                row![
                    text(txt)
                ]
                .align_items(Alignment::Center).into()
            }).collect()
        )
        .spacing(10);

        if state.is_finished {
            return column![
                row![
                    button("Back").on_press(Message::Back),
                    button("Send Cancel").on_press(Message::CancelTransfer)
                ].padding(4).spacing(8),
                
                text(if download { "Download" } else { "Upload" })
                    .width(Length::Fill)
                    .size(50)
                    .style(Color::from([0.7, 0.7, 0.7]))
                    .horizontal_alignment(alignment::Horizontal::Center),
                text("Completed")
                    .width(Length::Fill)
                    .size(30)
                    .horizontal_alignment(alignment::Horizontal::Center),

                log,
                horizontal_rule(2)

            ].spacing(5).padding(10).into()
        }   

    column![
        row![
            button("Back").on_press(Message::Back),
            button("Send Cancel").on_press(Message::CancelTransfer)
        ].padding(4).spacing(8),
        
        text(if download { "Download" } else { "Upload" })
            .width(Length::Fill)
            .size(50)
            .style(Color::from([0.7, 0.7, 0.7]))
            .horizontal_alignment(alignment::Horizontal::Center),

            row![
                column![
                    row![
                        text("Protocol:")
                        .width(Length::Units(left_size))
                        .style(Color::from([0.5, 0.5, 0.5]))
                        .horizontal_alignment(alignment::Horizontal::Right),
                        text(state.protocol_name.clone()),
                    ].padding(4).spacing(8),
                    row![
                        text("Check/size:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(check),
                    ].padding(4).spacing(8),
                    row![
                        text("State:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(current_state),
                    ].padding(4).spacing(8),
                ],
                horizontal_space(Length::Units(50)),
                column![
                    row![
                        text("Total errors:")
                        .width(Length::Units(left_size))
                        .style(Color::from([0.5, 0.5, 0.5]))
                        .horizontal_alignment(alignment::Horizontal::Right),
                        text(transfer_state.errors),
                    ].padding(4).spacing(8),
                    row![
                        text("Elapsed time:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(elapsed_time),
                    ].padding(4).spacing(8),
                    row![
                        text("Time left:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(format!("{:02}:{:02}", time_left.as_secs() / 60, time_left.as_secs() % 60))
                    ].padding(4).spacing(8),
                ]
            ],

            row![
                text("Current file name:")
                    .style(Color::from([0.5, 0.5, 0.5])),
                text(file_name),
                text("size:")
                    .style(Color::from([0.5, 0.5, 0.5])),
                text(bb.bytes(file_size as u64)),
            ].padding(4).spacing(8),

            progress_bar(0.0..=transfer_state.file_size as f32, transfer_state.bytes_transfered as f32),

            row![
                text(format!("{}% {}/{}", (transfer_state.bytes_transfered * 100) / max(1, transfer_state.file_size),  bb.bytes(transfer_state.bytes_transfered as u64), bb.bytes(transfer_state.file_size as u64))),
                text(if download { "received" } else { "sent"}).style(Color::from([0.5, 0.5, 0.5])),

                text("transfer rate:")
                .style(Color::from([0.5, 0.5, 0.5])),
                text(format!("{}", bb.bytes(bps as u64))),
                text("per second")
                .style(Color::from([0.5, 0.5, 0.5])),
            ].padding(4).spacing(8),

            row![
                text("Log:")
                .style(Color::from([0.5, 0.5, 0.5])),
                // horizontal_rule(2),
            ].padding(4).spacing(8),
            log,
            horizontal_rule(2)

        ].spacing(5).padding(10).into()
    } else {
        column![
            text("error"),
            button("Cancel")
                .on_press(Message::CancelTransfer)
        ].spacing(8).padding(10).into()
    }
}
