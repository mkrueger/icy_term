use chrono::{DateTime, Local};
use eframe::{
    egui::{self, Layout, RichText, ScrollArea, TextEdit, WidgetText},
    emath::NumExt,
    epaint::{Color32, FontFamily, FontId, Vec2},
};
use glow::CW;
use i18n_embed_fl::fl;
use rand::Rng;

use crate::address_mod::{self, store_phone_book, Address, Terminal};

use super::{main_window_mod::MainWindow, DEFAULT_MODES};

pub enum PhonebookFilter {
    All,
    Favourites,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AdressCategory {
    Server,
    Login,
    Terminal,
    Notes,
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
                ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.horizontal(|ui| {
                        let selected =
                            matches!(window.phonebook_filter, PhonebookFilter::Favourites);
                        let r: egui::Response = ui
                            .selectable_label(
                                selected,
                                RichText::new("★")
                                    .font(FontId::new(28.0, FontFamily::Proportional)),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(
                                    RichText::new(fl!(
                                        crate::LANGUAGE_LOADER,
                                        "phonebook-starred-items"
                                    ))
                                    .small(),
                                );
                            });

                        if r.clicked() {
                            window.phonebook_filter = if selected {
                                PhonebookFilter::All
                            } else {
                                PhonebookFilter::Favourites
                            };
                        }

                        ui.add(
                            TextEdit::singleline(&mut window.phonebook_filter_string)
                                .desired_width(f32::INFINITY)
                                .hint_text(RichText::new(fl!(
                                    crate::LANGUAGE_LOADER,
                                    "phonebook-filter-placeholder"
                                ))),
                        );

                        let r: egui::Response = ui
                            .button(
                                RichText::new("✖")
                                    .font(FontId::new(20.0, FontFamily::Proportional)),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(
                                    RichText::new(fl!(
                                        crate::LANGUAGE_LOADER,
                                        "phonebook-clear-filter"
                                    ))
                                    .small(),
                                );
                            });
                        if r.clicked() {
                            window.phonebook_filter_string = String::new();
                        }
                    });
                });
                ui.add_space(8.);
                render_list(window, ui);
                ui.add_space(8.);

                ui.with_layout(Layout::left_to_right(egui::Align::BOTTOM), |ui| {
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
                        window.selected_bbs = None;
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
                            window.selected_bbs.is_some(),
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

                    let connect_text =
                        WidgetText::from(fl!(crate::LANGUAGE_LOADER, "phonebook-connect-button"));
                    let connect_text_size = connect_text
                        .into_galley(ui, Some(false), 1000., egui::TextStyle::Button)
                        .size();

                    let cancel_text =
                        WidgetText::from(fl!(crate::LANGUAGE_LOADER, "phonebook-cancel-button"));
                    let cancel_text_size = cancel_text
                        .into_galley(ui, Some(false), 1000., egui::TextStyle::Button)
                        .size();

                    ui.add_space(
                        ui.available_size_before_wrap().x
                            - connect_text_size.x
                            - cancel_text_size.x
                            - 8.,
                    );

                    let r: egui::Response = ui.add(egui::Button::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "phonebook-cancel-button"
                    )));
                    if r.clicked() {
                        window.call_bbs_uuid(window.selected_bbs);
                    }

                    let r: egui::Response = ui.add(egui::Button::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "phonebook-connect-button"
                    )));
                    if r.clicked() {
                        window.call_bbs_uuid(window.selected_bbs);
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
    if window.selected_bbs.is_some() {
        let sav: Address = window.get_address_mut(window.selected_bbs).clone();
        view_edit_bbs(ui, window.get_address_mut(window.selected_bbs));
        if sav != *window.get_address_mut(window.selected_bbs) {
            store_phonebook(window);
        }
    } else {
        render_quick_connect(window, ui);
    }
}

pub fn store_phonebook(window: &MainWindow) {
    if let Err(err) = store_phone_book(&window.addresses) {
        eprintln!("{err}");
    }
}

fn render_quick_connect(window: &mut MainWindow, ui: &mut egui::Ui) {
    let adr = window.get_address_mut(window.selected_bbs);
    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut adr.address)
                .desired_width(ui.available_width() - 50.)
                .hint_text(fl!(crate::LANGUAGE_LOADER, "phonebook-connect-to"))
                .font(FontId::proportional(22.)),
        );
    });
    ui.add_space(8.);
    egui::Grid::new("some_unique_id")
        .num_columns(2)
        .spacing([4.0, 8.0])
        .min_row_height(24.)
        .show(ui, |ui| {
            // Protocol row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-protocol"
                )));
            });

            egui::ComboBox::from_id_source("combobox1")
                .selected_text(RichText::new(format!("{:?}", adr.protocol)))
                .show_ui(ui, |ui| {
                    for ct in &address_mod::Protocol::ALL {
                        let label = RichText::new(format!("{ct:?}"));
                        ui.selectable_value(&mut adr.protocol, *ct, label);
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

            egui::ComboBox::from_id_source("combobox2")
                .selected_text(RichText::new(format!("{}", adr.screen_mode)))
                .width(250.)
                .show_ui(ui, |ui| {
                    for mode in &DEFAULT_MODES {
                        if matches!(mode, super::ScreenMode::Default) {
                            ui.separator();
                            continue;
                        }
                        let label = RichText::new(format!("{mode}"));
                        ui.selectable_value(&mut adr.screen_mode, *mode, label);
                    }
                });
            ui.end_row();

            // Terminal type row
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-terminal_type"
                )));
            });
            egui::ComboBox::from_id_source("combobox3")
                .selected_text(RichText::new(format!("{}", adr.terminal_type)))
                .width(250.)
                .show_ui(ui, |ui| {
                    for t in &Terminal::ALL {
                        let label = RichText::new(format!("{t}"));
                        ui.selectable_value(&mut adr.terminal_type, *t, label);
                    }
                });
            ui.end_row();
        });
}

fn render_list(window: &mut MainWindow, ui: &mut egui::Ui) {
    let row_height = 18. * 2.;
    let addresses: Vec<Address> = if let PhonebookFilter::Favourites = window.phonebook_filter {
        window
            .addresses
            .iter()
            .filter(|a| a.is_favored && filter_bbs(window, a))
            .cloned()
            .collect()
    } else {
        window
            .addresses
            .iter()
            .filter(|a| filter_bbs(window, a))
            .cloned()
            .collect()
    };

    ScrollArea::vertical().show_rows(ui, row_height, addresses.len(), |ui, range| {
        for i in range.start..range.end {
            let addr = &addresses[i];
            ui.with_layout(ui.layout().with_cross_justify(true), |ui| {
                let show_quick_connect = window.phonebook_filter_string.is_empty()
                    && matches!(window.phonebook_filter, PhonebookFilter::All);
                let selected = match window.selected_bbs {
                    Some(uuid) => addr.uuid == uuid,
                    None => i == 0 && show_quick_connect,
                };
                let r = ui.add(if i == 0 && show_quick_connect {
                    let mut addr = AddressRow::new(
                        selected,
                        Address::new(fl!(crate::LANGUAGE_LOADER, "phonebook-connect-to-address")),
                    );
                    addr.centered = true;
                    addr
                } else {
                    AddressRow::new(selected, addr.clone())
                });

                if r.clicked() {
                    if i == 0 && show_quick_connect {
                        window.select_bbs(None);
                    } else {
                        window.select_bbs(Some(addr.uuid));
                    }
                }
                if r.double_clicked() {
                    window.call_bbs(i);
                }
            });
        }
    });
}

fn filter_bbs(window: &MainWindow, a: &Address) -> bool {
    if window.phonebook_filter_string.is_empty() {
        return true;
    }
    let lower = window.phonebook_filter_string.to_lowercase();
    a.system_name.to_lowercase().contains(lower.as_str())
        || a.address.to_lowercase().contains(lower.as_str())
}

fn view_edit_bbs(ui: &mut egui::Ui, adr: &mut crate::address_mod::Address) {
    // Name row

    ui.horizontal(|ui| {
        ui.add(
            TextEdit::singleline(&mut adr.system_name)
                .desired_width(f32::INFINITY)
                .hint_text(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "phonebook-name-placeholder"
                ))),
        );
        let text = if adr.is_favored {
            RichText::new("★")
                .font(FontId::new(20.0, FontFamily::Proportional))
                .color(Color32::YELLOW)
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
            let converted: DateTime<Local> = DateTime::from(*last_call);
            ui.label(
                converted
                    .format(fl!(crate::LANGUAGE_LOADER, "phonebook-date-format").as_str())
                    .to_string(),
            );
        }
        None => {
            ui.label(RichText::new(fl!(
                crate::LANGUAGE_LOADER,
                "phonebook-not-called"
            )));
        }
    }

    ui.horizontal(|ui| {
        ui.label("✆");
        ui.label(adr.number_of_calls.to_string());
        ui.add_space(16.);
        /*
        ui.label("⮉");
        ui.label(adr.upladed_bytes.to_string());
        ui.add_space(16.);

        ui.label("⮋");
        ui.label(adr.downloaded_bytes.to_string());
        ui.add_space(16.);

        ui.label("⏰");
        ui.label(format!(
            "{} min",
            adr.overall_duration.num_minutes().to_string()
        ));*/
    });

    // Tab
    ui.add_space(8.);
    ui.separator();
    ui.horizontal(|ui| {
        ui.add_space(16.);

        ui.selectable_value(&mut adr.adress_category, AdressCategory::Server, "Server");
        ui.add_space(8.);

        ui.selectable_value(&mut adr.adress_category, AdressCategory::Login, "Login");
        ui.add_space(8.);

        ui.selectable_value(
            &mut adr.adress_category,
            AdressCategory::Terminal,
            "Terminal",
        );
        ui.add_space(8.);

        ui.selectable_value(&mut adr.adress_category, AdressCategory::Notes, "Comment");
    });
    ui.separator();
    ui.add_space(8.);

    match adr.adress_category {
        AdressCategory::Server => {
            egui::Grid::new("some_unique_id")
                .num_columns(2)
                .spacing([4.0, 8.0])
                .min_row_height(24.)
                .show(ui, |ui| {
                    // Addreess row
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "phonebook-address"
                        )));
                    });
                    ui.add(TextEdit::singleline(&mut adr.address));
                    ui.end_row();

                    // Protocol row
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "phonebook-protocol"
                        )));
                    });

                    egui::ComboBox::from_id_source("combobox1")
                        .selected_text(RichText::new(format!("{:?}", adr.protocol)))
                        .show_ui(ui, |ui| {
                            for ct in &address_mod::Protocol::ALL {
                                let label = RichText::new(format!("{ct:?}"));
                                ui.selectable_value(&mut adr.protocol, *ct, label);
                            }
                        });
                    ui.end_row();
                });
        }
        AdressCategory::Login => {
            egui::Grid::new("some_unique_id")
                .num_columns(2)
                .spacing([4.0, 8.0])
                .min_row_height(24.)
                .show(ui, |ui| {
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
        }
        AdressCategory::Terminal => {
            egui::Grid::new("some_unique_id")
                .num_columns(2)
                .spacing([4.0, 8.0])
                .min_row_height(24.)
                .show(ui, |ui| {
                    // Screen mode row
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "phonebook-screen_mode"
                        )));
                    });

                    egui::ComboBox::from_id_source("combobox2")
                        .selected_text(RichText::new(format!("{}", adr.screen_mode)))
                        .width(250.)
                        .show_ui(ui, |ui| {
                            for mode in &DEFAULT_MODES {
                                if matches!(mode, super::ScreenMode::Default) {
                                    ui.separator();
                                    continue;
                                }
                                let label = RichText::new(format!("{mode}"));
                                ui.selectable_value(&mut adr.screen_mode, *mode, label);
                            }
                        });
                    ui.end_row();

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "phonebook-terminal_type"
                        )));
                    });
                    egui::ComboBox::from_id_source("combobox3")
                        .selected_text(RichText::new(format!("{}", adr.terminal_type)))
                        .width(250.)
                        .show_ui(ui, |ui| {
                            for t in &Terminal::ALL {
                                let label = RichText::new(format!("{t}"));
                                ui.selectable_value(&mut adr.terminal_type, *t, label);
                            }
                        });
                    ui.end_row();
                });
        }

        AdressCategory::Notes => {
            ui.add(TextEdit::multiline(&mut adr.comment).desired_width(f32::INFINITY));
        }
    }

    let converted: DateTime<Local> = DateTime::from(adr.created);
    ui.with_layout(Layout::left_to_right(egui::Align::BOTTOM), |ui| {
        ui.label(format!(
            "Created at {}",
            converted.format(fl!(crate::LANGUAGE_LOADER, "phonebook-date-format").as_str())
        ));
    });
}

pub struct AddressRow {
    selected: bool,
    pub centered: bool,
    addr: Address,
}

impl AddressRow {
    pub fn new(selected: bool, addr: Address) -> Self {
        Self {
            selected,
            centered: false,
            addr,
        }
    }
}

impl egui::Widget for AddressRow {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let Self {
            selected,
            centered,
            addr,
        } = self;

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding + Vec2::new(0.0, 8.0);

        let wrap_width = ui.available_width() - total_extra.x;
        let star_text = WidgetText::from(
            RichText::new("★")
                .font(FontId::new(14.0, FontFamily::Proportional))
                .color(Color32::YELLOW),
        );
        let star_text: egui::widget_text::WidgetTextGalley =
            star_text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);
        let star_text_size = star_text.size();

        let mut rt = RichText::new(addr.system_name.clone());
        if !centered {
            rt = rt.color(Color32::WHITE);
        }
        let name_text = WidgetText::from(rt);
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
            if centered {
                let text_pos = rect.left_top()
                    + Vec2::new(
                        (rect.width() - name_text_size.x) / 2.0,
                        rect.height() / 2.0 - name_text_size.y / 2.0,
                    );
                name_text.paint_with_visuals(ui.painter(), text_pos, &visuals);
            } else {
                let text_pos = rect.left_top() + button_padding;
                name_text.paint_with_visuals(ui.painter(), text_pos, &visuals);

                let text_pos = rect.left_top() + button_padding + Vec2::new(0.0, name_text_size.y);
                addr_text.paint_with_visuals(ui.painter(), text_pos, &visuals);

                if addr.is_favored {
                    let text_pos =
                        rect.right_top() - button_padding - Vec2::new(star_text_size.x, -2.);
                    star_text.paint_with_visuals(ui.painter(), text_pos, &visuals);
                }
            }
        }

        response
    }
}
