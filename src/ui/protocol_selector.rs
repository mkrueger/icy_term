use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBody, TableBuilder};
use i18n_embed_fl::fl;

use crate::protocol::TransferType;

use super::main_window_mod::{MainWindow, MainWindowMode};

fn create_button_row(
    window: &mut MainWindow,
    body: &mut TableBody,
    protocol: TransferType,
    download: bool,
    title: &'static str,
    descr: String,
) {
    body.row(30., |mut row| {
        row.col(|ui| {
            if ui.button(RichText::new(title)).clicked() {
                window.initiate_file_transfer(protocol, download);
            }
        });
        row.col(|ui| {
            ui.label(RichText::new(descr));
        });
    });
}

pub fn view_selector(
    window: &mut MainWindow,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    download: bool,
) {
    let mut open = true;
    let title = RichText::new(if download {
        fl!(crate::LANGUAGE_LOADER, "protocol-select-download")
    } else {
        fl!(crate::LANGUAGE_LOADER, "protocol-select-upload")
    });

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
                    TransferType::ZModem,
                    download,
                    "Zmodem",
                    fl!(crate::LANGUAGE_LOADER, "protocol-zmodem-description"),
                );
                create_button_row(
                    window,
                    &mut body,
                    TransferType::ZedZap,
                    download,
                    "ZedZap",
                    fl!(crate::LANGUAGE_LOADER, "protocol-zmodem8k-description"),
                );
                create_button_row(
                    window,
                    &mut body,
                    TransferType::XModem,
                    download,
                    "Xmodem",
                    fl!(crate::LANGUAGE_LOADER, "protocol-xmodem-description"),
                );
                create_button_row(
                    window,
                    &mut body,
                    TransferType::XModem1k,
                    download,
                    "Xmodem 1k",
                    fl!(crate::LANGUAGE_LOADER, "protocol-xmodem1k-description"),
                );
                create_button_row(
                    window,
                    &mut body,
                    TransferType::XModem1kG,
                    download,
                    "Xmodem 1k-G",
                    fl!(crate::LANGUAGE_LOADER, "protocol-xmodem1kG-description"),
                );
                create_button_row(
                    window,
                    &mut body,
                    TransferType::YModem,
                    download,
                    "Ymodem",
                    fl!(crate::LANGUAGE_LOADER, "protocol-ymodem-description"),
                );
                create_button_row(
                    window,
                    &mut body,
                    TransferType::YModemG,
                    download,
                    "Ymodem-G",
                    fl!(crate::LANGUAGE_LOADER, "protocol-ymodemg-description"),
                );
            });
        });

    if !open {
        window.mode = MainWindowMode::ShowTerminal;
    }
}
