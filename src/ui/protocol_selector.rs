use eframe::{
    egui::{self, RichText},
    epaint::FontId,
};
use egui_extras::{Column, TableBody, TableBuilder};

use crate::protocol::ProtocolType;

use super::main_window::{MainWindow, MainWindowMode};

fn create_button_row(
    window: &mut MainWindow,
    body: &mut TableBody,
    protocol: ProtocolType,
    download: bool,
    title: &'static str,
    descr: &'static str,
) {
    let text_style = FontId::proportional(22.);
    body.row(30., |mut row| {
        row.col(|ui| {
            if ui
                .button(RichText::new(title).font(text_style.clone()))
                .clicked()
            {
                window.initiate_file_transfer(protocol, download);
            }
        });
        row.col(|ui| {
            ui.label(RichText::new(descr).font(text_style.clone()));
        });
    });
}

pub fn view_protocol_selector(
    window: &mut MainWindow,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    download: bool,
) {
    let mut open = true;
    let text_style = FontId::proportional(26.);
    let title = RichText::new(format!(
        "Select {} protocol",
        if download { "download" } else { "upload" }
    ))
    .font(text_style);

    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            let table = TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::initial(100.0).at_least(100.0))
                .column(Column::remainder().at_least(60.0))
                .resizable(false);
            table.body(|mut body| {
                create_button_row(
                    window,
                    &mut body,
                    ProtocolType::ZModem,
                    download,
                    "Zmodem",
                    "The standard protocol",
                );
                create_button_row(
                    window,
                    &mut body,
                    ProtocolType::ZedZap,
                    download,
                    "ZedZap",
                    "8k Zmodem",
                );
                create_button_row(
                    window,
                    &mut body,
                    ProtocolType::XModem,
                    download,
                    "Xmodem",
                    "Outdated protocol",
                );
                create_button_row(
                    window,
                    &mut body,
                    ProtocolType::XModem1k,
                    download,
                    "Xmodem 1k",
                    "Rarely used anymore",
                );
                create_button_row(
                    window,
                    &mut body,
                    ProtocolType::XModem1kG,
                    download,
                    "Xmodem 1k-G",
                    "Does that even exist?",
                );
                create_button_row(
                    window,
                    &mut body,
                    ProtocolType::YModem,
                    download,
                    "Ymodem",
                    "Ok but Zmodem is better",
                );
                create_button_row(
                    window,
                    &mut body,
                    ProtocolType::YModemG,
                    download,
                    "Ymodem-G",
                    "A fast Ymodem variant",
                );
            });
        });

    if !open {
        window.mode = MainWindowMode::ShowTerminal;
    }
}
