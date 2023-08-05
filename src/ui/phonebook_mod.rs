use eframe::{
    egui::{self, Layout, RichText, ScrollArea, TextEdit},
    epaint::{FontId, Vec2},
};
use i18n_embed_fl::fl;
use rand::Rng;

use crate::address_mod::{self, store_phone_book, Address, Terminal};

use super::{main_window_mod::MainWindow, DEFAULT_MODES};

pub fn view_phonebook(window: &mut MainWindow, ctx: &egui::Context) {
    let img_size = Vec2::new(24., 24.);

    let mut open = true;
    let mut r = ctx.available_rect().shrink(80.);
    r.set_top(r.top() - 40.0);
    let w = egui::Window::new(fl!(crate::LANGUAGE_LOADER, "phonebook-dialog-title"))
        .default_width(600.0)
        .default_height(400.0)
        .collapsible(false)
        .vscroll(false)
        .resizable(true)
        .fixed_rect(r)
        .open(&mut open);

    w.show(ctx, |ui| {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    let r = ui
                        .add(egui::ImageButton::new(
                            super::CALL_SVG.texture_id(ctx),
                            img_size,
                        ))
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-call"))
                                    .small(),
                            );
                        });

                    if r.clicked() {
                        window.call_bbs(window.selected_bbs);
                    }

                    let r: egui::Response = ui
                        .add(egui::ImageButton::new(
                            super::ADD_SVG.texture_id(ctx),
                            img_size,
                        ))
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-add")).small(),
                            );
                        });

                    if r.clicked() {
                        window.addresses.push(Address::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "phonebook-new_bbs"
                        )));
                        window.selected_bbs = window.addresses.len() - 1;
                    }

                    ui.add_space(ui.available_size_before_wrap().x);

                    let r = ui
                        .add_enabled(
                            window.selected_bbs > 0,
                            egui::ImageButton::new(super::DELETE_SVG.texture_id(ctx), img_size),
                        )
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-delete"))
                                    .small(),
                            );
                        });

                    if r.clicked() {
                        window.delete_selected_address();
                    }
                });
            });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0)
            .show_inside(ui, |ui| {
                render_list(window, ui);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            show_content(window, ui);
        });
    });

    if !open {
        window.show_terminal();
    }
}

fn show_content(window: &mut MainWindow, ui: &mut egui::Ui) {
    if window.selected_bbs > 0 {
        let sav: Address = window.addresses[window.selected_bbs].clone();
        view_edit_bbs(ui, &mut window.addresses[window.selected_bbs]);
        if sav != window.addresses[window.selected_bbs] {
            if let Err(err) = store_phone_book(&window.addresses) {
                eprintln!("{err}");
            }
        }
    } else {
        let adr = &mut window.addresses[window.selected_bbs];
        ui.horizontal(|ui| {
            ui.label(RichText::new(fl!(
                crate::LANGUAGE_LOADER,
                "phonebook-address"
            )));

            ui.add(
                TextEdit::singleline(&mut adr.address)
                    .desired_width(ui.available_width() - 50.)
                    .hint_text(fl!(crate::LANGUAGE_LOADER, "phonebook-connect-to"))
                    .font(FontId::proportional(22.)),
            );
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(fl!(
                crate::LANGUAGE_LOADER,
                "phonebook-screen_mode"
            )));

            egui::ComboBox::from_id_source("combobox2")
                .selected_text(RichText::new(format!(
                    "{:?}",
                    adr.screen_mode.unwrap_or(super::ScreenMode::NotSet)
                )))
                .show_ui(ui, |ui| {
                    for mode in &DEFAULT_MODES {
                        let label = RichText::new(format!("{mode:?}"));
                        ui.selectable_value(&mut adr.screen_mode, Some(*mode), label);
                    }
                });
            ui.label(RichText::new(fl!(
                crate::LANGUAGE_LOADER,
                "phonebook-terminal_type"
            )));
            egui::ComboBox::from_id_source("combobox3")
                .selected_text(RichText::new(format!("{:?}", adr.terminal_type)))
                .show_ui(ui, |ui| {
                    for t in &Terminal::ALL {
                        let label = RichText::new(format!("{t:?}"));
                        ui.selectable_value(&mut adr.terminal_type, *t, label);
                    }
                });
        });
    }
}

fn render_list(window: &mut MainWindow, ui: &mut egui::Ui) {
    let row_height = 18.;
    ScrollArea::vertical().show_rows(ui, row_height, window.addresses.len(), |ui, range| {
        for i in range.start..range.end {
            let addr = window.addresses[i].clone();
            ui.horizontal(|ui| {
                let r = ui.selectable_label(
                    i == window.selected_bbs,
                    if i == 0 {
                        fl!(crate::LANGUAGE_LOADER, "phonebook-connect-to-address")
                    } else {
                        addr.system_name
                    },
                );
                if r.clicked() {
                    window.select_bbs(i);
                }
                if r.double_clicked() {
                    window.call_bbs(i);
                }
            });
        }
    });
}

fn view_edit_bbs(ui: &mut egui::Ui, adr: &mut crate::address_mod::Address) {
    egui::Grid::new("some_unique_id")
        .num_columns(2)
        .spacing([4.0, 8.0])
        .min_row_height(24.)
        .show(ui, |ui| {
            // Name row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-name")));
            });
            ui.add(TextEdit::singleline(&mut adr.system_name).desired_width(f32::INFINITY));
            ui.end_row();

            // Addreess row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-address"
                )));
            });

            ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(TextEdit::singleline(&mut adr.address));

                egui::ComboBox::from_id_source("combobox1")
                    .selected_text(RichText::new(format!("{:?}", adr.connection_type)))
                    .show_ui(ui, |ui| {
                        for ct in &address_mod::ConnectionType::ALL {
                            let label = RichText::new(format!("{ct:?}"));
                            ui.selectable_value(&mut adr.connection_type, *ct, label);
                        }
                    });
            });
            ui.end_row();

            // User row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-user")));
            });
            ui.add(TextEdit::singleline(&mut adr.user_name).desired_width(f32::INFINITY));
            ui.end_row();

            // Password row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-password"
                )));
            });
            ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(TextEdit::singleline(&mut adr.password));
                if ui
                    .button(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "phonebook-generate"
                    )))
                    .clicked()
                {
                    let mut rng = rand::thread_rng();
                    let mut pw = String::new();
                    for _ in 0..16 {
                        pw.push(unsafe {
                            char::from_u32_unchecked(rng.gen_range(b'0'..b'z') as u32)
                        });
                    }
                    adr.password = pw;
                }
            });
            ui.end_row();

            // Screen mode row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-screen_mode"
                )));
            });

            ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_source("combobox2")
                    .selected_text(RichText::new(format!(
                        "{:?}",
                        adr.screen_mode.unwrap_or(super::ScreenMode::NotSet)
                    )))
                    .show_ui(ui, |ui| {
                        for mode in &DEFAULT_MODES {
                            let label = RichText::new(format!("{mode:?}"));
                            ui.selectable_value(&mut adr.screen_mode, Some(*mode), label);
                        }
                    });
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-terminal_type"
                )));
                egui::ComboBox::from_id_source("combobox3")
                    .selected_text(RichText::new(format!("{:?}", adr.terminal_type)))
                    .show_ui(ui, |ui| {
                        for t in &Terminal::ALL {
                            let label = RichText::new(format!("{t:?}"));
                            ui.selectable_value(&mut adr.terminal_type, *t, label);
                        }
                    });
            });
            ui.end_row();

            // Autologin row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-autologin"
                )));
            });
            ui.add(TextEdit::singleline(&mut adr.auto_login).desired_width(f32::INFINITY));
            ui.end_row();

            // Comment row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-comment"
                )));
            });
            ui.add(TextEdit::singleline(&mut adr.comment).desired_width(f32::INFINITY));
            ui.end_row();
        });
}
