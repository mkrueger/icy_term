use eframe::{
    egui::{self, ScrollArea, TextEdit, RichText},
    epaint::{FontId, Vec2, Color32},
};
use rand::Rng;

use crate::{address::{Address, self, store_phone_book, Terminal}};

use super::{main_window::MainWindow, DEFAULT_MODES};

pub fn view_phonebook(window: &mut MainWindow, ctx: &egui::Context, _frame: &mut eframe::Frame) {

    egui::TopBottomPanel::top("top_panel")
    .default_height(36.0)
    .height_range(36.0..=36.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            let img_size = Vec2::new(24., 24.);
            if ui
                .add(egui::ImageButton::new(
                    super::CALL_SVG.texture_id(ctx),
                    img_size,
                ))
                .clicked()
            {
                window.call_bbs(0);
            }

            ui.add(
                TextEdit::singleline(&mut window.addresses[0].address)
                    .desired_width(ui.available_width() - 50.)
                    .hint_text("Quick connect to…")
                    .font(FontId::proportional(22.)),
            );

            if ui
                .add(egui::ImageButton::new(
                    super::SETTINGS_SVG.texture_id(ctx),
                    img_size,
                ))
                .clicked()
            {
                window.show_settings(true);
            }
        });
    });
    egui::SidePanel::left("left")
    .default_width(200.0)
    .width_range(200.0..=200.0)
    .show(ctx, |ui| {
        let row_height = 18.;
        if window.addresses.len() > 1 {
            ScrollArea::vertical().show_rows(
                ui,
                row_height,
                window.addresses.len() - 1,
                |ui, range| {
                    for i in range.start..range.end {
                        let addr = window.addresses[i + 1].clone();
                        let img_size = Vec2::new(row_height, row_height);

                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::ImageButton::new(
                                    super::CALL_SVG.texture_id(ctx),
                                    img_size,
                                    ).frame(false)
                                )
                                .clicked()
                            {
                                window.call_bbs(i + 1);
                                return;
                            }
                            let text_style = FontId::proportional(row_height + 2.);
                            let mut text = RichText::new(addr.system_name.clone()).font(text_style);
                            if i == window.selected_bbs {
                                text = text.color(Color32::WHITE);
                            }
                            if ui.button(text).clicked() {
                                window.select_bbs(i + 1);
                            }
                        });
                    }
                },
            );
        } else { 
            let text_style = FontId::proportional(22.);
            ui.label(RichText::new("No BBSes").font(text_style));
        }

        ui.horizontal(|ui| {
            ui.horizontal(|ui| {
                let img_size = Vec2::new(22., 22.);
                if ui
                    .add(egui::ImageButton::new(
                        super::DELETE_SVG.texture_id(ctx),
                        img_size,
                    ))
                    .clicked()
                {
                    window.delete_selected_address();
                }
                if ui
                    .add(egui::ImageButton::new(
                        super::ADD_SVG.texture_id(ctx),
                        img_size,
                    ))
                    .clicked()
                {
                    window.addresses.push(Address::new("New BBS".to_string()));
                    window.selected_bbs = window.addresses.len() - 1;
                }
            });  
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        if window.selected_bbs > 0 {
            let sav = window.addresses[window.selected_bbs].clone();
            ui.vertical(|ui| {
                view_edit_bbs(ui, &mut window.addresses[window.selected_bbs]);
            });
            if sav != window.addresses[window.selected_bbs] {
                if let Err(err) = store_phone_book(&window.addresses) {
                    eprintln!("{}", err);
                }
            }
        } else {
            let text_style = FontId::proportional(22.);
            ui.label(RichText::new("No selection").font(text_style));
        }
    });
}

fn view_edit_bbs(ui: &mut egui::Ui, adr: &mut crate::address::Address) {
    let text_style = FontId::proportional(22.);

    egui::Grid::new("some_unique_id")
    .spacing(Vec2::new(5., 8.))
    
    .show(ui, |ui| {
        ui.label(RichText::new("Name").font(text_style.clone()));
        ui.add(
            TextEdit::singleline(&mut adr.system_name)
                .desired_width(f32::INFINITY)
                .font(text_style.clone()),
        );
        ui.end_row();
    
        ui.label(RichText::new("Address").font(text_style.clone()));
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut adr.address)
                .font(text_style.clone()),
            );

            egui::ComboBox::from_id_source("combobox1")
            .selected_text(RichText::new(format!("{:?}", adr.connection_type)).font(text_style.clone()))
            .show_ui(ui, |ui| {
                for ct in &address::ConnectionType::ALL {
                    let label = RichText::new(format!("{:?}", ct)).font(text_style.clone());
                    ui.selectable_value(&mut adr.connection_type, *ct, label);
                }
            });
        });
        ui.end_row();

        ui.label(RichText::new("User").font(text_style.clone()));
        ui.add(
            TextEdit::singleline(&mut adr.user_name)
                .desired_width(f32::INFINITY)
                .font(text_style.clone()),
        );
        ui.end_row();

        ui.label(RichText::new("Password").font(text_style.clone()));
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut adr.password)
                    .font(text_style.clone()),
            );
            if ui.button(RichText::new("Generate").font(text_style.clone())).clicked() {
                let mut rng = rand::thread_rng();                
                let mut pw = String::new();
                for _ in 0..16 {
                    pw.push(unsafe{char::from_u32_unchecked(rng.gen_range(b'0'..b'z') as u32) });
                }
                adr.password = pw;
            }
        });
        ui.end_row();

        ui.label(RichText::new("Screen Mode").font(text_style.clone()));
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_source("combobox2")
            .selected_text(RichText::new(format!("{:?}", adr.screen_mode)).font(text_style.clone()))
            .show_ui(ui, |ui| {
                for mode in &DEFAULT_MODES {
                    let label = RichText::new(format!("{:?}", mode)).font(text_style.clone());
                    ui.selectable_value(&mut adr.screen_mode, Some(*mode), label);
                }
            });
            ui.label(RichText::new("Terminal type").font(text_style.clone()));
            egui::ComboBox::from_id_source("combobox3")
            .selected_text(RichText::new(format!("{:?}", adr.terminal_type)).font(text_style.clone()))
            .show_ui(ui, |ui| {
                for t in &Terminal::ALL {
                    let label = RichText::new(format!("{:?}", t)).font(text_style.clone());
                    ui.selectable_value(&mut adr.terminal_type, *t, label);
                }
            });
        });
        ui.end_row();

        ui.label(RichText::new("Autologin String").font(text_style.clone()));
        ui.add(
            TextEdit::singleline(&mut adr.auto_login)
                .desired_width(f32::INFINITY)
                .font(text_style.clone()),
        );
        ui.end_row();

        ui.label(RichText::new("Comment").font(text_style.clone()));
        ui.add(
            TextEdit::singleline(&mut adr.comment)
                .desired_width(f32::INFINITY)
                .font(text_style.clone()),
        );
        ui.end_row();
    });

}
