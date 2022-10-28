use iced::widget::{ column, row, progress_bar, text};
use iced::{
    Element
};
use crate::protocol::{Protocol};
use super::main_window::{Message};


pub fn view_file_transfer<'a,T: Protocol>(protocol: &T, download: bool) ->Element<'a, Message> {
    let s = protocol.get_current_state();
/*     if s.is_none() {
        return;
    }*/
    let state = s.unwrap();

    let transfer_state = if download { state.recieve_state } else { state.send_state }.unwrap();
    
    /* 
    button("Cancel")
            .on_press(Message::Back)*/
    column![
        row![
            text("Protocol:"),
            text(protocol.get_name()),
        ].padding(4)
        .spacing(8),
        row![
            text("Check/size:"),
            text(transfer_state.check_size.clone()),
        ].padding(4)
        .spacing(8),
        row![
            text("File name:"),
            text(transfer_state.get_file_name()),
            text("File size:"),
            text(transfer_state._get_file_size()),
        ].padding(4)
        .spacing(8),
        row![
            text("Bytes send:"),
            text(transfer_state.bytes_transfered),
        ].padding(4)
        .spacing(8),
        progress_bar(0.0..=transfer_state.get_total_bytes() as f32, transfer_state.bytes_transfered as f32),
        text(state.current_state)
    ].spacing(8).into()
}
