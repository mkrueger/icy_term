use std::cmp::{max};
use std::time::{Duration};

use gabi::BytesConfig;
use iced::widget::{ column, row, progress_bar, text, button};
use iced::{
    Element
};
use crate::protocol::{ TransferState };
use super::Message;

pub fn view_file_transfer<'a>(state: &TransferState, download: bool) ->Element<'a, Message> {

    if let Some(transfer_state) = if download { state.recieve_state.as_ref() } else { state.send_state.as_ref() } {
        let check = transfer_state.check_size.clone();
        let file_name = transfer_state.file_name.clone();
        let file_size  = transfer_state.file_size;
        let current_state = state.current_state.to_string();
        let engine_state = transfer_state.engine_state.clone();

        let bps  = transfer_state.get_bps();
        let bytes_left = transfer_state.file_size - transfer_state.bytes_transfered;
        let time_left = Duration::from_secs(bytes_left as u64 / max(1, bps));

        let bb = BytesConfig::default();
        column![
            row![
                text("Protocol:"),
                text(state.protocol_name.clone()),
            ].padding(4)
            .spacing(8),
            row![
                text("Check/size:"),
                text(check),
            ].padding(4)
            .spacing(8),
            row![
                text("Current file name:"),
                text(file_name),
                text("File size:"),
                text(bb.bytes(file_size as u64)),
            ].padding(4)
            .spacing(8),
            row![
                text(format!("{} bytes", bb.bytes(transfer_state.bytes_transfered as u64))),
                text(if download { "received" } else { "sent"}),
                text(format!("transfer rate: {} per second ~time left: {:02}:{:02}", bb.bytes(bps as u64), time_left.as_secs() / 60, time_left.as_secs() % 60))
            ].padding(4)
            .spacing(8),
            text(current_state),
            progress_bar(0.0..=transfer_state.file_size as f32, transfer_state.bytes_transfered as f32),
            text(engine_state),
            button("Cancel")
                .on_press(Message::CancelTransfer),
            button("Back")
                .on_press(Message::Back)
        ].spacing(8).padding(10).into()
    } else {
        column![
            text("error"),
            button("Cancel")
                .on_press(Message::CancelTransfer)
        ].spacing(8).padding(10).into()
    }
}
