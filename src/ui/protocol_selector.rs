use eframe::egui::{self, RichText};
use i18n_embed_fl::fl;

use crate::protocol::TransferType;

use super::{MainWindow, MainWindowMode};

fn create_button_row(
    ui: &mut egui::Ui,
    window: &mut MainWindow,
    protocol: TransferType,
    download: bool,
    title: &'static str,
    descr: &str,
) {
    ui.with_layout(ui.layout().with_cross_justify(true), |ui| {
        if ui
            .selectable_label(
                false,
                RichText::new(format!("{:15}{}", title, descr)).family(egui::FontFamily::Monospace),
            )
            .clicked()
        {
            window.initiate_file_transfer(protocol, download);
        }
    });
}

pub fn view_selector(
    window: &mut MainWindow,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
    download: bool,
) {
    let mut open = true;
    let mut close = false;

    if ctx.input(|i| i.key_down(egui::Key::Escape)) {
        open = false;
    }

    let title = RichText::new(if download {
        fl!(crate::LANGUAGE_LOADER, "protocol-select-download")
    } else {
        fl!(crate::LANGUAGE_LOADER, "protocol-select-upload")
    });

    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .min_width(550.)
        .resizable(false)
        .show(ctx, |ui| {
            ui.add_space(8.0);

            create_button_row(
                ui,
                window,
                TransferType::ZModem,
                download,
                "Zmodem",
                &fl!(crate::LANGUAGE_LOADER, "protocol-zmodem-description"),
            );
            create_button_row(
                ui,
                window,
                TransferType::ZedZap,
                download,
                "ZedZap",
                &fl!(crate::LANGUAGE_LOADER, "protocol-zmodem8k-description"),
            );
            create_button_row(
                ui,
                window,
                TransferType::XModem,
                download,
                "Xmodem",
                &fl!(crate::LANGUAGE_LOADER, "protocol-xmodem-description"),
            );
            create_button_row(
                ui,
                window,
                TransferType::XModem1k,
                download,
                "Xmodem 1k",
                &fl!(crate::LANGUAGE_LOADER, "protocol-xmodem1k-description"),
            );
            create_button_row(
                ui,
                window,
                TransferType::XModem1kG,
                download,
                "Xmodem 1k-G",
                &fl!(crate::LANGUAGE_LOADER, "protocol-xmodem1kG-description"),
            );
            create_button_row(
                ui,
                window,
                TransferType::YModem,
                download,
                "Ymodem",
                &fl!(crate::LANGUAGE_LOADER, "protocol-ymodem-description"),
            );
            create_button_row(
                ui,
                window,
                TransferType::YModemG,
                download,
                "Ymodem-G",
                &fl!(crate::LANGUAGE_LOADER, "protocol-ymodemg-description"),
            );
            ui.add_space(8.0);
            ui.separator();
            ui.vertical_centered(|ui| {
                if ui
                    .button(&fl!(crate::LANGUAGE_LOADER, "phonebook-cancel-button"))
                    .clicked()
                {
                    close = true;
                }
            });
        });

    if !open || close {
        window.mode = MainWindowMode::ShowTerminal;
    }
}
