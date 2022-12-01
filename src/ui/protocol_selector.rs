use eframe::egui::{self};
use egui_extras::{Size, TableBuilder, TableBody};

use crate::protocol::ProtocolType;

use super::main_window::{MainWindow, MainWindowMode};

fn create_button_row(window: &mut MainWindow, body: &mut TableBody, protocol: ProtocolType, download: bool, title: &'static str, descr: &'static str) {
    body.row(18., |mut row| {
        row.col(|ui| {
            if ui.button(title).clicked() {
                window.initiate_file_transfer(protocol, download);
            }
        });
        row.col(|ui| {
            ui.label(descr);
        });
    });
}

pub fn view_protocol_selector(window: &mut MainWindow, ctx: &egui::Context, _frame: &mut eframe::Frame, download: bool) {
    let mut open = true;
    egui::Window::new(format!("Select {} protocol", if download { "download" } else { "upload" } ))
    .open(&mut open)
    .collapsible(false)
    .resizable(false)
    .show(ctx, |ui| {
        let table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Size::initial(100.0).at_least(100.0))
            .column(Size::remainder().at_least(60.0))
            .resizable(false);
        table
            .body(|mut body| {
                create_button_row(window, &mut body, ProtocolType::ZModem, download, "Zmodem", "The standard protocol");
                create_button_row(window, &mut body, ProtocolType::ZedZap, download, "ZedZap", "8k Zmodem");
                create_button_row(window, &mut body, ProtocolType::XModem, download, "Xmodem", "Outdated protocol");
                create_button_row(window, &mut body, ProtocolType::XModem1k, download, "Xmodem 1k", "Rarely used anymore");
                create_button_row(window, &mut body, ProtocolType::XModem1kG, download, "Xmodem 1k-G", "Does that even exist?");
                create_button_row(window, &mut body, ProtocolType::YModem, download, "Ymodem", "Ok but Zmodem is better");
                create_button_row(window, &mut body, ProtocolType::YModemG, download, "Ymodem-G", "A fast Ymodem variant");
            });
    });

    if !open {
        window.mode = MainWindowMode::ShowTerminal;
    }
}
