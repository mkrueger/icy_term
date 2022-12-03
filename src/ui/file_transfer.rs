use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use eframe::epaint::Color32;
use gabi::BytesConfig;
use eframe::egui::{self, ProgressBar};
use egui_extras::{Size, TableBuilder, Column};

use crate::protocol::TransferState;

pub fn view_file_transfer(ctx: &egui::Context, frame: &mut eframe::Frame, transfer_state: &Arc<Mutex<TransferState>>, download: bool) -> bool {
    let mut open = true;
    egui::Window::new(if download { "Download" } else { "Upload" } )
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let state = transfer_state.lock().unwrap();
        let transfer_info = if download {
            &state.recieve_state
        } else {
            &state.send_state
        };

        let check = transfer_info.check_size.clone();
        let file_name = transfer_info.file_name.clone();
        let current_state = state.current_state.to_string();

        let bps = transfer_info.get_bps();
        let bytes_left = transfer_info.file_size.saturating_sub(transfer_info.bytes_transfered);
        let time_left = Duration::from_secs(bytes_left as u64 / max(1, bps));

        let bb = BytesConfig::default();

        let elapsed_time = SystemTime::now().duration_since(state.start_time).unwrap();
        let elapsed_time = format!(
            "{:02}:{:02}",
            elapsed_time.as_secs() / 60,
            elapsed_time.as_secs() % 60
        );
/* 
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
        .spacing(10);*/

        if state.is_finished {
            ui.label("Completed");
        }

        let table = TableBuilder::new(ui)
            .striped(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::auto())
            .resizable(false);

        table.body(|mut body| {
            body.row(18., |mut row| {
                row.col(|ui| {
                    ui.label("Protocol:");
                    ui.colored_label( Color32::WHITE, state.protocol_name.clone());
                });

                row.col(|ui| {
                    ui.label("Total errors:");
                    ui.colored_label( Color32::WHITE, transfer_info.errors.to_string());
                });
            });

            body.row(18., |mut row| {
                row.col(|ui| {
                    ui.label("Check/size:");
                    ui.colored_label( Color32::WHITE, check);
                });

                row.col(|ui| {
                    ui.label("Elapsed time:");
                    ui.colored_label( Color32::WHITE, elapsed_time);
                });
            });

            body.row(18., |mut row| {
                row.col(|ui| {
                    ui.label("State:");
                    ui.colored_label( Color32::WHITE, current_state);
                });

                row.col(|ui| {
                    ui.label("Time left:");
                    ui.colored_label( Color32::WHITE, format!(
                        "{:02}:{:02}",
                        time_left.as_secs() / 60,
                        time_left.as_secs() % 60
                    ));
                });
            });
        });

        ui.horizontal(|ui| {
            ui.label("File:");
            ui.colored_label( Color32::WHITE, file_name);
        });
        ui.add(ProgressBar::new(transfer_info.bytes_transfered as f32 / transfer_info.file_size as f32).text(format!(
            "{}% {}/{}",
            (transfer_info.bytes_transfered * 100) / max(1, transfer_info.file_size),
            bb.bytes(transfer_info.bytes_transfered as u64),
            bb.bytes(transfer_info.file_size as u64)
        )));
        ui.horizontal(|ui| {
            ui.label("transfer rate:");
            ui.colored_label( Color32::WHITE, format!("{}", bb.bytes(bps as u64)));
            ui.colored_label( Color32::WHITE, "per second");
        });
    });
    open
}
