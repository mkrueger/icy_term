use iced::{alignment, Alignment, Color, Element, Length};
use std::cmp::max;
use std::time::{Duration, SystemTime};

use gabi::BytesConfig;
use iced::widget::{ column, horizontal_rule, horizontal_space, progress_bar, row, text};

use super::Message;
use crate::protocol::TransferState;

pub fn view_file_transfer<'a>(state: &TransferState, download: bool) -> Element<'a, Message> {
    if let Some(transfer_state) = if download {
        state.recieve_state.as_ref()
    } else {
        state.send_state.as_ref()
    } {
        let check = transfer_state.check_size.clone();
        let file_name = transfer_state.file_name.clone();
        let file_size = transfer_state.file_size;
        let current_state = state.current_state.to_string();

        let bps = transfer_state.get_bps();
        let bytes_left = transfer_state.file_size.saturating_sub(transfer_state.bytes_transfered);
        let time_left = Duration::from_secs(bytes_left as u64 / max(1, bps));

        let bb = BytesConfig::default();

        let left_size = 100;

        let elapsed_time = SystemTime::now().duration_since(state.start_time).unwrap();
        let elapsed_time = format!(
            "{:02}:{:02}",
            elapsed_time.as_secs() / 60,
            elapsed_time.as_secs() % 60
        );

        let log = column(
            transfer_state
                .output_log
                .iter()
                .rev()
                .take(1)
                .rev()
                .map(|txt| row![text(txt)].align_items(Alignment::Center).into())
                .collect(),
        )
        .spacing(10);

        if state.is_finished {
            return column![
                text("Completed")
                    .width(Length::Fill)
                    .size(30)
                    .horizontal_alignment(alignment::Horizontal::Center),
                log,
                horizontal_rule(2)
            ]
            .spacing(5)
            .padding(10)
            .into();
        }

        column![
            row![
                column![
                    row![
                        text("Protocol:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(state.protocol_name.clone()),
                    ]
                    .padding(4)
                    .spacing(8),
                    row![
                        text("Check/size:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(check),
                    ]
                    .padding(4)
                    .spacing(8),
                    row![
                        text("State:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(current_state),
                    ]
                    .padding(4)
                    .spacing(8),
                ],
                horizontal_space(Length::Units(50)),
                column![
                    row![
                        text("Total errors:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(transfer_state.errors),
                    ]
                    .padding(4)
                    .spacing(8),
                    row![
                        text("Elapsed time:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(elapsed_time),
                    ]
                    .padding(4)
                    .spacing(8),
                    row![
                        text("Time left:")
                            .width(Length::Units(left_size))
                            .style(Color::from([0.5, 0.5, 0.5]))
                            .horizontal_alignment(alignment::Horizontal::Right),
                        text(format!(
                            "{:02}:{:02}",
                            time_left.as_secs() / 60,
                            time_left.as_secs() % 60
                        ))
                    ]
                    .padding(4)
                    .spacing(8),
                ]
            ],
            row![
                text("File:").style(Color::from([0.5, 0.5, 0.5])),
                text(file_name),
            ]
            .padding(4)
            .spacing(8),
            progress_bar(
                0.0..=transfer_state.file_size as f32,
                transfer_state.bytes_transfered as f32
            ),
            row![
                text(format!(
                    "{}% {}/{}",
                    (transfer_state.bytes_transfered * 100) / max(1, transfer_state.file_size),
                    bb.bytes(transfer_state.bytes_transfered as u64),
                    bb.bytes(transfer_state.file_size as u64)
                )),
                text(if download { "received" } else { "sent" })
                    .style(Color::from([0.5, 0.5, 0.5])),
                text("transfer rate:").style(Color::from([0.5, 0.5, 0.5])),
                text(format!("{}", bb.bytes(bps as u64))),
                text("per second").style(Color::from([0.5, 0.5, 0.5])),
            ]
            .padding(4)
            .spacing(8),
            log
        ]
        .spacing(5)
        .padding(10)
        .into()
    } else {
        column![
            text("error"),
        ]
        .spacing(8)
        .padding(10)
        .into()
    }
}
