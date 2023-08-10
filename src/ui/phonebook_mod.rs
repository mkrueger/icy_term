use eframe::{
    egui::{self, Layout, RichText, ScrollArea, TextEdit, WidgetText},
    emath::NumExt,
    epaint::{Color32, FontFamily, FontId, Vec2},
};
use i18n_embed_fl::fl;
use rand::Rng;

use crate::address_mod::{self, store_phone_book, Address, Terminal};

use super::{main_window_mod::MainWindow, DEFAULT_MODES};

pub enum PhonebookFilter {
    All,
    Favourites
}


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdressCategory {
    Server,
    Terminal,
    Notes
}

pub fn view_phonebook(window: &mut MainWindow, ctx: &egui::Context) {
    let img_size = Vec2::new(24., 24.);

    let mut open = true;
    let mut r = ctx.available_rect().shrink(80.);
    r.set_top(r.top() - 40.0);
    let w = egui::Window::new("")
        .default_width(600.0)
        .default_height(400.0)
        .collapsible(false)
        .vscroll(false)
        .resizable(true)
        .title_bar(false)
        .fixed_rect(r)
        .open(&mut open);

    w.show(ctx, |ui| {

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .exact_width(350.0 + 16.0)
            .show_inside(ui, |ui| {

                ui.with_layout(Layout::right_to_left(egui::Align::TOP), |ui| {

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
                });
                ui.add_space(8.);
                render_list(window, ui);
                ui.add_space(8.);

                ui.with_layout(Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                    let r: egui::Response = ui.selectable_label(matches!(window.phonebook_filter, PhonebookFilter::All), RichText::new("📞").font(FontId::new(20.0, FontFamily::Proportional)));
                    if r.clicked() {
                        window.phonebook_filter = PhonebookFilter::All;
                    }

                    let r: egui::Response = ui.selectable_label(matches!(window.phonebook_filter, PhonebookFilter::Favourites), RichText::new("★").font(FontId::new(20.0, FontFamily::Proportional)));
                    if r.clicked() {
                        window.phonebook_filter = PhonebookFilter::Favourites;
                    }
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.);
                ui.horizontal(|ui| {

                    let r: egui::Response = ui
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

                    let connect_text = WidgetText::from(fl!(crate::LANGUAGE_LOADER, "phonebook-connect-button"));
                    let connect_text_size = connect_text.into_galley(ui, Some(false), 1000., egui::TextStyle::Button).size();
            
                    let cancel_text = WidgetText::from(fl!(crate::LANGUAGE_LOADER, "phonebook-cancel-button"));
                    let cancel_text_size = cancel_text.into_galley(ui, Some(false), 1000., egui::TextStyle::Button).size();

                    ui.add_space(ui.available_size_before_wrap().x  - connect_text_size.x - cancel_text_size.x - 8.);

                    let r: egui::Response = ui.add(egui::Button::new(fl!(crate::LANGUAGE_LOADER, "phonebook-cancel-button")));
                    if r.clicked() {
                        window.call_bbs(window.selected_bbs);
                    }

                    let r: egui::Response = ui.add(egui::Button::new(fl!(crate::LANGUAGE_LOADER, "phonebook-connect-button")));
                    if r.clicked() {
                        window.call_bbs(window.selected_bbs);
                    }

                });
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
                .selected_text(RichText::new(format!("{:?}", adr.screen_mode)))
                .show_ui(ui, |ui| {
                    for mode in &DEFAULT_MODES {
                        let label = RichText::new(format!("{mode:?}"));
                        ui.selectable_value(&mut adr.screen_mode, *mode, label);
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
    let row_height = 18. * 2.;

    let addresses = if let PhonebookFilter::Favourites = window.phonebook_filter {
        window.addresses.iter().filter(|a| a.is_favored).cloned().collect()
    } else {
        window.addresses.clone()
    };

    ScrollArea::vertical().show_rows(ui, row_height, addresses.len(), |ui, range| {
        for i in range.start..range.end {
            let addr = &addresses[i];
            ui.with_layout(ui.layout().with_cross_justify(true), |ui| {
                let r = ui.add(AddressRow::new(
                    i == window.selected_bbs,
                    if i == 0 && matches!(window.phonebook_filter, PhonebookFilter::All) {
                        Address::new(fl!(crate::LANGUAGE_LOADER, "phonebook-connect-to-address"))
                    } else {
                        addr.clone()
                    },
                ));

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
    // Name row

    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut adr.system_name).desired_width(f32::INFINITY).hint_text(RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-name-placeholder"))));
        let text = if adr.is_favored {
            RichText::new("★").font(FontId::new(20.0, FontFamily::Proportional)).color(Color32::YELLOW)
        } else {
            RichText::new("☆").font(FontId::new(20.0, FontFamily::Proportional))
        };
    
        if ui.selectable_label(false, text).clicked() {
            adr.is_favored = !adr.is_favored;
        }
    });

    ui.add_space(8.);


    match &adr.last_call {
        Some(last_call) => {
            ui.label(last_call.to_string());
        }
        None => {
            ui.label(RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-not-called")));
        }
    }

    ui.add_space(8.);

    ui.separator();
    ui.horizontal(|ui| {
        ui.selectable_value(&mut adr.adress_category, AdressCategory::Server, "Server");
        ui.selectable_value(&mut adr.adress_category, AdressCategory::Terminal, "Terminal");
        ui.selectable_value(&mut adr.adress_category, AdressCategory::Notes, "Comment");
    });
    ui.separator();


    match adr.adress_category {
        AdressCategory::Server => {
            egui::Grid::new("some_unique_id")
        .num_columns(2)
        .spacing([4.0, 8.0])
        .min_row_height(24.)
        .show(ui, |ui| {

            // Addreess row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-address")));
            });
            ui.add(TextEdit::singleline(&mut adr.address));
            ui.end_row();

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(crate::LANGUAGE_LOADER, "phonebook-protocol")));
            });
          
            egui::ComboBox::from_id_source("combobox1")
                .selected_text(RichText::new(format!("{:?}", adr.connection_type)))
                .show_ui(ui, |ui| {
                    for ct in &address_mod::ConnectionType::ALL {
                        let label = RichText::new(format!("{ct:?}"));
                        ui.selectable_value(&mut adr.connection_type, *ct, label);
                    }
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
                    .selected_text(RichText::new(format!("{:?}", adr.screen_mode)))
                    .show_ui(ui, |ui| {
                        for mode in &DEFAULT_MODES {
                            let label = RichText::new(format!("{mode:?}"));
                            ui.selectable_value(&mut adr.screen_mode, *mode, label);
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

        });
        },
        AdressCategory::Terminal => {
            egui::Grid::new("some_unique_id")
            .num_columns(2)
            .spacing([4.0, 8.0])
            .min_row_height(24.)
            .show(ui, |ui| {
    
            });    
        }

        AdressCategory::Notes => {
            egui::Grid::new("some_unique_id")
        .num_columns(2)
        .spacing([4.0, 8.0])
        .min_row_height(24.)
        .show(ui, |ui| {
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
    }

    
}

pub struct AddressRow {
    selected: bool,
    addr: Address,
}

impl AddressRow {
    pub fn new(selected: bool, addr: Address) -> Self {
        Self { selected, addr }
    }
}

impl egui::Widget for AddressRow {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self { selected, addr } = self;

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding + Vec2::new(0.0, 8.0);

        let wrap_width = ui.available_width() - total_extra.x;
        let star_text = WidgetText::from(RichText::new("★").font(FontId::new(14.0, FontFamily::Proportional)).color(Color32::YELLOW));
        let star_text: egui::widget_text::WidgetTextGalley = star_text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);
        let star_text_size = star_text.size();


        let name_text =
        WidgetText::from(RichText::new(addr.system_name.clone()).color(Color32::WHITE));
        let name_text = name_text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);
        let name_text_size = name_text.size();



        let addr_text = WidgetText::from(
            RichText::new(addr.address.clone()).font(FontId::new(14.0, FontFamily::Proportional)),
        );
        let addr_text = addr_text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);

        let mut desired_size = total_extra + name_text.size() + Vec2::new(0.0, addr_text.size().y);
        desired_size.x = 350.0;
        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_at_least(desired_size, egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                selected,
                name_text.text(),
            )
        });

        if ui.is_rect_visible(response.rect) {
            let visuals = ui.style().interact_selectable(&response, selected);

            if selected || response.hovered() || response.highlighted() || response.has_focus() {
                let rect = rect.expand(visuals.expansion);

                ui.painter().rect(
                    rect,
                    visuals.rounding,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                );
            }

            let text_pos = rect.left_top() + button_padding;
            name_text.paint_with_visuals(ui.painter(), text_pos, &visuals);

            let text_pos = rect.left_top() + button_padding + Vec2::new(0.0, name_text_size.y);
            addr_text.paint_with_visuals(ui.painter(), text_pos, &visuals);

            if addr.is_favored {
                let text_pos = rect.right_top() - button_padding - Vec2::new(star_text_size.x, -2.);
                star_text.paint_with_visuals(ui.painter(), text_pos, &visuals);
            }
        }

        response
    }
}
