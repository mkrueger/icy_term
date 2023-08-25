use chrono::{DateTime, Local};
use eframe::{
    egui::{self, Layout, RichText, ScrollArea, TextEdit, WidgetText},
    emath::NumExt,
    epaint::{FontFamily, FontId, Vec2},
};
use egui::{Id, Rect};
use i18n_embed_fl::fl;
use icy_engine::ansi::{BaudEmulation, MusicOption};

use crate::{
    addresses::{self, Address, Terminal},
    ui::{MainWindow, MainWindowMode, ScreenMode, DEFAULT_MODES},
    util::Rng,
    AddressBook,
};

#[derive(Default)]
pub enum DialingDirectoryFilter {
    #[default]
    All,
    Favourites,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AddressCategory {
    Server,
    Login,
    Terminal,
    Notes,
}

const phone_list_width: f32 = 220.0;
const PROTOCOL_COMBOBOX_WIDTH: f32 = 180.0;

#[derive(Default)]
pub struct DialogState {
    pub addresses: AddressBook,

    pub cur_addr: usize,
    pub selected_bbs: Option<usize>,
    pub scroll_address_list_to_bottom: bool,
    pub dialing_directory_filter: DialingDirectoryFilter,
    pub dialing_directory_filter_string: String,
    rng: Rng,
    show_passwords: bool,
}

impl DialogState {
    pub fn get_address_mut(&mut self, uuid: Option<usize>) -> &mut Address {
        if uuid.is_none() {
            return &mut self.addresses.addresses[0];
        }

        let uuid = uuid.unwrap();
        for (i, adr) in self.addresses.addresses.iter().enumerate() {
            if adr.id == uuid {
                return &mut self.addresses.addresses[i];
            }
        }

        &mut self.addresses.addresses[0]
    }
    pub fn delete_bbs(&mut self, uuid: usize) {
        for (i, adr) in self.addresses.addresses.iter().enumerate() {
            if adr.id == uuid {
                self.addresses.addresses.remove(i);
                break;
            }
        }
        let _ = self.addresses.store_phone_book();
        //check_error!(self, r, false);
    }

    pub fn select_bbs(&mut self, uuid: Option<usize>) {
        self.selected_bbs = uuid;
    }

    fn show_content(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.selected_bbs.is_some() {
            let sav: Address = self.get_address_mut(self.selected_bbs).clone();
            self.view_edit_bbs(ctx, ui);
            if sav != *self.get_address_mut(self.selected_bbs) {
                self.store_dialing_directory();
            }
        } else {
            self.render_quick_connect(ui);
        }
    }

    pub fn store_dialing_directory(&mut self) {
        if let Err(err) = self.addresses.store_phone_book() {
            log::error!("Failed to store dialing_directory: {err}");
        }
    }

    fn render_quick_connect(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.get_address_mut(self.selected_bbs).address)
                    .id(Id::new("dialing_directory-connect-to"))
                    .desired_width(ui.available_width() - 50.)
                    .hint_text(fl!(crate::LANGUAGE_LOADER, "dialing_directory-connect-to"))
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
                        "dialing_directory-protocol"
                    )));
                });

                egui::ComboBox::from_id_source("combobox1")
                    .selected_text(RichText::new(format!(
                        "{}",
                        self.get_address_mut(self.selected_bbs).protocol
                    )))
                    .width(PROTOCOL_COMBOBOX_WIDTH)
                    .show_ui(ui, |ui| {
                        for prot in &addresses::Protocol::ALL {
                            let label = RichText::new(format!("{prot}"));
                            ui.selectable_value(
                                &mut self.get_address_mut(self.selected_bbs).protocol,
                                *prot,
                                label,
                            );
                        }
                    });
                ui.end_row();

                // Screen mode row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-screen_mode"
                    )));
                });

                egui::ComboBox::from_id_source("combobox2")
                    .selected_text(RichText::new(format!(
                        "{}",
                        self.get_address_mut(self.selected_bbs).screen_mode
                    )))
                    .width(250.)
                    .show_ui(ui, |ui| {
                        for mode in &DEFAULT_MODES {
                            if matches!(mode, ScreenMode::Default) {
                                ui.separator();
                                continue;
                            }
                            let label = RichText::new(format!("{mode}"));
                            ui.selectable_value(
                                &mut self.get_address_mut(self.selected_bbs).screen_mode,
                                *mode,
                                label,
                            );
                        }
                    });
                ui.end_row();

                // Terminal type row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-terminal_type"
                    )));
                });
                egui::ComboBox::from_id_source("combobox3")
                    .selected_text(RichText::new(format!(
                        "{}",
                        self.get_address_mut(self.selected_bbs).terminal_type
                    )))
                    .width(250.)
                    .show_ui(ui, |ui| {
                        for t in &Terminal::ALL {
                            let label = RichText::new(format!("{t}"));
                            ui.selectable_value(
                                &mut self.get_address_mut(self.selected_bbs).terminal_type,
                                *t,
                                label,
                            );
                        }
                    });
                ui.end_row();

                // Baud emulation
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-baud-emulation"
                    )))
                });

                egui::ComboBox::from_id_source("combobox5")
                    .selected_text(RichText::new(format!(
                        "{}",
                        self.get_address_mut(self.selected_bbs).baud_emulation
                    )))
                    .width(250.)
                    .show_ui(ui, |ui| {
                        for b in &BaudEmulation::OPTIONS {
                            let label = RichText::new(format!("{b}"));
                            ui.selectable_value(
                                &mut self.get_address_mut(self.selected_bbs).baud_emulation,
                                *b,
                                label,
                            );
                        }
                    });
                ui.end_row();
            });
        ui.add_space(50.);

        if self.addresses.write_lock {
            let msg = fl!(crate::LANGUAGE_LOADER, "dialing_directory-version-warning");
            ui.label(RichText::new(msg).color(ui.ctx().style().visuals.warn_fg_color));
        } else {
            let r: egui::Response = ui.button(
                RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "dialing_directory-add-bbs-button"
                ))
                .font(FontId::new(20.0, FontFamily::Proportional)),
            );

            if r.clicked() {
                let mut cloned_addr = self.addresses.addresses[0].clone();
                cloned_addr.id = Address::new(String::new()).id; // create a new id
                cloned_addr.system_name = cloned_addr.address.clone(); // set a system name
                self.select_bbs(Some(cloned_addr.id));
                self.addresses.addresses.push(cloned_addr);
                self.dialing_directory_filter = DialingDirectoryFilter::All;
                self.scroll_address_list_to_bottom = true;
            }
        }
    }

    fn render_list(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        // let row_height = 18. * 2.;
        let addresses: Vec<Address> =
            if let DialingDirectoryFilter::Favourites = self.dialing_directory_filter {
                self.addresses
                    .addresses
                    .iter()
                    .filter(|a| a.is_favored && self.filter_bbs(a))
                    .cloned()
                    .collect()
            } else {
                self.addresses
                    .addresses
                    .iter()
                    .filter(|a| self.filter_bbs(a))
                    .cloned()
                    .collect()
            };

        if addresses.is_empty() {
            ui.label(fl!(crate::LANGUAGE_LOADER, "dialing_directory-no-entries"));
        }

        let mut result = None;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    (0..addresses.len()).for_each(|i| {
                        let addr = &addresses[i];
                        ui.with_layout(ui.layout().with_cross_justify(true), |ui| {
                            let show_quick_connect =
                                self.dialing_directory_filter_string.is_empty()
                                    && matches!(
                                        self.dialing_directory_filter,
                                        DialingDirectoryFilter::All
                                    );
                            let selected = match self.selected_bbs {
                                Some(uuid) => addr.id == uuid,
                                None => i == 0 && show_quick_connect,
                            };
                            let r = ui.add(if i == 0 && show_quick_connect {
                                let mut addr = AddressRow::new(
                                    selected,
                                    Address::new(fl!(
                                        crate::LANGUAGE_LOADER,
                                        "dialing_directory-connect-to-address"
                                    )),
                                );
                                addr.centered = true;
                                addr
                            } else {
                                AddressRow::new(selected, addr.clone())
                            });

                            if r.clicked() {
                                if i == 0 && show_quick_connect {
                                    self.select_bbs(None);
                                } else {
                                    self.select_bbs(Some(addr.id));
                                }
                            }
                            if r.double_clicked() {
                                result = Some(addr.id);
                            }
                        });
                    });
                });
                if self.scroll_address_list_to_bottom {
                    self.scroll_address_list_to_bottom = false;
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                }
            });
        result
    }

    fn filter_bbs(&self, a: &Address) -> bool {
        if self.dialing_directory_filter_string.is_empty() {
            return true;
        }
        let lower = self.dialing_directory_filter_string.to_lowercase();
        a.system_name.to_lowercase().contains(lower.as_str())
            || a.address.to_lowercase().contains(lower.as_str())
    }

    #[allow(clippy::range_plus_one)]
    fn view_edit_bbs(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Name row

        ui.horizontal(|ui| {
            let adr = self.get_address_mut(self.selected_bbs);
            ui.add(
                TextEdit::singleline(&mut adr.system_name)
                    .id(Id::new("dialing_directory-name-placeholder"))
                    .desired_width(f32::INFINITY)
                    .hint_text(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-name-placeholder"
                    ))),
            );
            let text = if adr.is_favored {
                RichText::new("★")
                    .font(FontId::new(20.0, FontFamily::Proportional))
                    .color(ctx.style().visuals.warn_fg_color)
            } else {
                RichText::new("☆").font(FontId::new(20.0, FontFamily::Proportional))
            };

            if ui.selectable_label(false, text).clicked() {
                adr.is_favored = !adr.is_favored;
            }
        });

        ui.add_space(8.);

        match &self.get_address_mut(self.selected_bbs).last_call {
            Some(last_call) => {
                let converted: DateTime<Local> = DateTime::from(*last_call);
                ui.label(
                    converted
                        .format(
                            fl!(
                                crate::LANGUAGE_LOADER,
                                "dialing_directory-last-call-date-format"
                            )
                            .as_str(),
                        )
                        .to_string(),
                );
            }
            None => {
                ui.label(RichText::new(fl!(
                    crate::LANGUAGE_LOADER,
                    "dialing_directory-not-called"
                )));
            }
        }

        ui.horizontal(|ui| {
            let adr = self.get_address_mut(self.selected_bbs);

            ui.label("✆");
            ui.label(adr.number_of_calls.to_string());
            ui.add_space(16.);
            /*
            ui.label("⮉");
            ui.label(adr.uploaded_bytes.to_string());
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
            let adr = self.get_address_mut(self.selected_bbs);

            ui.add_space(16.);

            ui.selectable_value(&mut adr.address_category, AddressCategory::Server, "Server");
            ui.add_space(8.);

            ui.selectable_value(&mut adr.address_category, AddressCategory::Login, "Login");
            ui.add_space(8.);

            ui.selectable_value(
                &mut adr.address_category,
                AddressCategory::Terminal,
                "Terminal",
            );
            ui.add_space(8.);

            ui.selectable_value(&mut adr.address_category, AddressCategory::Notes, "Comment");
        });
        ui.separator();
        ui.add_space(8.);

        match self.get_address_mut(self.selected_bbs).address_category {
            AddressCategory::Server => {
                self.render_server_catogery(ui);
            }
            AddressCategory::Login => {
                self.render_login_category(ui);
            }
            AddressCategory::Terminal => {
                self.render_terminal_category(ui);
            }

            AddressCategory::Notes => {
                ui.add(
                    TextEdit::multiline(&mut self.get_address_mut(self.selected_bbs).comment)
                        .desired_width(f32::INFINITY),
                );
            }
        }

        let converted: DateTime<Local> =
            DateTime::from(self.get_address_mut(self.selected_bbs).created);
        ui.with_layout(Layout::left_to_right(egui::Align::BOTTOM), |ui| {
            let str = fl!(
                crate::LANGUAGE_LOADER,
                "dialing_directory-created-at-date-format"
            );
            ui.label(converted.format(str.as_str()).to_string());
        });
    }

    const MUSIC_OPTIONS: [MusicOption; 4] = [
        MusicOption::Off,
        MusicOption::Banana,
        MusicOption::Conflicting,
        MusicOption::Both,
    ];

    fn render_terminal_category(&mut self, ui: &mut egui::Ui) {
        let adr = self.get_address_mut(self.selected_bbs);
        egui::Grid::new("some_unique_id")
            .num_columns(2)
            .spacing([4.0, 8.0])
            .min_row_height(24.)
            .show(ui, |ui| {
                // Screen mode row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-screen_mode"
                    )));
                });

                egui::ComboBox::from_id_source("combobox2")
                    .selected_text(RichText::new(format!("{}", adr.screen_mode)))
                    .width(250.)
                    .show_ui(ui, |ui| {
                        for mode in &DEFAULT_MODES {
                            if matches!(mode, ScreenMode::Default) {
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
                        "dialing_directory-terminal_type"
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

                if adr.terminal_type == Terminal::Ansi {
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "dialing_directory-music-option"
                        )));
                    });
                    egui::ComboBox::from_id_source("combobox4")
                        .selected_text(RichText::new(format!("{}", adr.ansi_music)))
                        .width(250.)
                        .show_ui(ui, |ui| {
                            for t in &DialogState::MUSIC_OPTIONS {
                                let label = RichText::new(format!("{t}"));
                                ui.selectable_value(&mut adr.ansi_music, *t, label);
                            }
                        });
                    ui.end_row();
                }

                // Baud emulation
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-baud-emulation"
                    )))
                });

                egui::ComboBox::from_id_source("combobox5")
                    .selected_text(RichText::new(format!("{}", adr.baud_emulation)))
                    .width(250.)
                    .show_ui(ui, |ui| {
                        for b in &BaudEmulation::OPTIONS {
                            let label = RichText::new(format!("{b}"));
                            ui.selectable_value(&mut adr.baud_emulation, *b, label);
                        }
                    });
                ui.end_row();
            });
    }

    fn render_login_category(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("some_unique_id")
            .num_columns(2)
            .spacing([4.0, 8.0])
            .min_row_height(24.)
            .show(ui, |ui| {
                // User row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-user"
                    )));
                });
                ui.add(
                    TextEdit::singleline(&mut self.get_address_mut(self.selected_bbs).user_name)
                        .desired_width(f32::INFINITY),
                );
                ui.end_row();

                // Password row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-password"
                    )));
                });
                ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                    let pw = self.show_passwords;
                    ui.add(
                        TextEdit::singleline(&mut self.get_address_mut(self.selected_bbs).password)
                            .password(!pw),
                    );

                    if ui.selectable_label(self.show_passwords, "👁").clicked() {
                        self.show_passwords = !self.show_passwords;
                    }

                    if ui
                        .button(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "dialing_directory-generate"
                        )))
                        .clicked()
                    {
                        let mut pw = String::new();
                        for _ in 0..16 {
                            pw.push(unsafe {
                                char::from_u32_unchecked(self.rng.gen_range(b'0'..=b'z'))
                            });
                        }
                        self.get_address_mut(self.selected_bbs).password = pw;
                    }
                });
                ui.end_row();

                // Autologin row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-autologin"
                    )));
                });
                ui.add(
                    TextEdit::singleline(&mut self.get_address_mut(self.selected_bbs).auto_login)
                        .desired_width(f32::INFINITY),
                );
                ui.end_row();
                ui.label("");

                ui.checkbox(
                    &mut self
                        .get_address_mut(self.selected_bbs)
                        .override_iemsi_settings,
                    "Custom IEMSI login data",
                );
                ui.end_row();

                if self
                    .get_address_mut(self.selected_bbs)
                    .override_iemsi_settings
                {
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "dialing_directory-user"
                        )));
                    });
                    ui.add(
                        TextEdit::singleline(
                            &mut self.get_address_mut(self.selected_bbs).iemsi_user,
                        )
                        .desired_width(f32::INFINITY),
                    );
                    ui.end_row();

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(fl!(
                            crate::LANGUAGE_LOADER,
                            "dialing_directory-password"
                        )));
                    });
                    ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                        let pw = self.show_passwords;
                        ui.add(
                            TextEdit::singleline(
                                &mut self.get_address_mut(self.selected_bbs).iemsi_password,
                            )
                            .password(!pw),
                        );

                        if ui.selectable_label(self.show_passwords, "👁").clicked() {
                            self.show_passwords = !self.show_passwords;
                        }
                    });
                }
            });
    }

    fn render_server_catogery(&mut self, ui: &mut egui::Ui) {
        let adr = self.get_address_mut(self.selected_bbs);
        egui::Grid::new("some_unique_id")
            .num_columns(2)
            .spacing([4.0, 8.0])
            .min_row_height(24.)
            .show(ui, |ui| {
                // Addreess row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-address"
                    )));
                });
                ui.add(TextEdit::singleline(&mut adr.address));
                ui.end_row();

                // Protocol row
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-protocol"
                    )));
                });

                egui::ComboBox::from_id_source("combobox1")
                    .selected_text(RichText::new(format!("{}", adr.protocol)))
                    .width(PROTOCOL_COMBOBOX_WIDTH)
                    .show_ui(ui, |ui| {
                        for prot in &addresses::Protocol::ALL {
                            let label = RichText::new(format!("{prot}"));
                            ui.selectable_value(&mut adr.protocol, *prot, label);
                        }
                    });
                ui.end_row();
            });
    }

    pub(crate) fn new(addresses: AddressBook) -> Self {
        Self {
            addresses,
            ..Default::default()
        }
    }
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
                .color(ui.ctx().style().visuals.warn_fg_color),
        );
        let star_text: egui::widget_text::WidgetTextGalley =
            star_text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);
        let star_text_size = star_text.size();

        let rt = RichText::new(addr.system_name.clone())
            .font(FontId::new(16., FontFamily::Proportional))
            .strong();

        let name_text = WidgetText::from(rt);
        let name_text = name_text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);
        let name_text_size = name_text.size();

        let addr_text = WidgetText::from(
            RichText::new(addr.address.clone()).font(FontId::new(12.0, FontFamily::Monospace)),
        );
        let addr_text = addr_text.into_galley(ui, Some(false), wrap_width, egui::TextStyle::Button);

        let mut desired_size = total_extra + name_text.size() + Vec2::new(0.0, addr_text.size().y);
        desired_size.x = phone_list_width;
        desired_size.y = desired_size
            .y
            .at_least(ui.spacing().interact_size.y)
            .floor();
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

pub fn view_dialing_directory(window: &mut MainWindow, ctx: &egui::Context) {
    let mut open = true;
    let available_rect = ctx.available_rect();
    let bounds = 16.0;
    let width = (available_rect.width() - bounds * 2. - 81.).min(900.);
    let height = (available_rect.height() - available_rect.top() - bounds * 2.).min(580.);
    let x_pos = available_rect.left() + (available_rect.width() - width).max(0.) / 2.;
    let y_pos = 20. + (available_rect.height() - height).max(0.) / 2.;
    if ctx.input(|i| i.key_down(egui::Key::Escape)) {
        open = false;
    }
    let w = egui::Window::new("")
        .collapsible(false)
        .vscroll(false)
        .resizable(true)
        .title_bar(false)
        .fixed_rect(Rect::from_min_size(
            egui::Pos2::new(x_pos, y_pos),
            Vec2::new(width, height),
        ))
        .open(&mut open);

    w.show(ctx, |ui| {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .exact_width(phone_list_width + 16.0)
            .show_inside(ui, |ui| {
                ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.horizontal(|ui| {
                        let selected = matches!(
                            window.dialing_directory_dialog.dialing_directory_filter,
                            DialingDirectoryFilter::Favourites
                        );
                        let r: egui::Response = ui
                            .selectable_label(
                                selected,
                                RichText::new("★")
                                    .font(FontId::new(22.0, FontFamily::Proportional)),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(
                                    RichText::new(fl!(
                                        crate::LANGUAGE_LOADER,
                                        "dialing_directory-starred-items"
                                    ))
                                    .small(),
                                );
                            });

                        if r.clicked() {
                            window.dialing_directory_dialog.dialing_directory_filter = if selected {
                                DialingDirectoryFilter::All
                            } else {
                                DialingDirectoryFilter::Favourites
                            };
                        }

                        ui.add(
                            TextEdit::singleline(
                                &mut window
                                    .dialing_directory_dialog
                                    .dialing_directory_filter_string,
                            )
                            .desired_width(f32::INFINITY)
                            .hint_text(RichText::new(fl!(
                                crate::LANGUAGE_LOADER,
                                "dialing_directory-filter-placeholder"
                            ))),
                        );

                        let r: egui::Response = ui
                            .button(
                                RichText::new("✖")
                                    .font(FontId::new(16.0, FontFamily::Proportional)),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(
                                    RichText::new(fl!(
                                        crate::LANGUAGE_LOADER,
                                        "dialing_directory-clear-filter"
                                    ))
                                    .small(),
                                );
                            });
                        if r.clicked() {
                            window
                                .dialing_directory_dialog
                                .dialing_directory_filter_string = String::new();
                        }
                    });
                });
                ui.add_space(8.);
                if let Some(uuid) = window.dialing_directory_dialog.render_list(ui) {
                    window.call_bbs_uuid(Some(uuid));
                }
                ui.add_space(8.);
                if !window.dialing_directory_dialog.addresses.write_lock {
                    ui.with_layout(Layout::left_to_right(egui::Align::BOTTOM), |ui| {
                        let r: egui::Response = ui
                            .button(
                                RichText::new("➕")
                                    .font(FontId::new(20.0, FontFamily::Proportional)),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(
                                    RichText::new(fl!(
                                        crate::LANGUAGE_LOADER,
                                        "dialing_directory-add"
                                    ))
                                    .small(),
                                );
                            });

                        if r.clicked() {
                            let adr = Address::new(fl!(
                                crate::LANGUAGE_LOADER,
                                "dialing_directory-new_bbs"
                            ));
                            window.dialing_directory_dialog.select_bbs(Some(adr.id));
                            window
                                .dialing_directory_dialog
                                .addresses
                                .addresses
                                .push(adr);
                            window.dialing_directory_dialog.dialing_directory_filter =
                                DialingDirectoryFilter::All;
                            window
                                .dialing_directory_dialog
                                .scroll_address_list_to_bottom = true;
                        }
                    });
                }
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    let r: egui::Response = ui
                        .add_enabled(
                            window.dialing_directory_dialog.selected_bbs.is_some(),
                            egui::Button::new(
                                RichText::new("🗑")
                                    .font(FontId::new(26.0, FontFamily::Proportional)),
                            ),
                        )
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(
                                    crate::LANGUAGE_LOADER,
                                    "dialing_directory-delete"
                                ))
                                .small(),
                            );
                        });

                    if r.clicked() {
                        if let Some(uuid) = window.dialing_directory_dialog.selected_bbs {
                            window.mode = MainWindowMode::DeleteSelectedAddress(uuid);
                        }
                    }

                    let connect_text = WidgetText::from(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-connect-button"
                    ));
                    let connect_text_size = connect_text
                        .into_galley(ui, Some(false), 1000., egui::TextStyle::Button)
                        .size();

                    let cancel_text = WidgetText::from(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-cancel-button"
                    ));
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
                        "dialing_directory-cancel-button"
                    )));
                    if r.clicked() {
                        window.show_terminal();
                    }

                    let r: egui::Response = ui.add(egui::Button::new(fl!(
                        crate::LANGUAGE_LOADER,
                        "dialing_directory-connect-button"
                    )));
                    if r.clicked() {
                        window.call_bbs_uuid(window.dialing_directory_dialog.selected_bbs);
                    }
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            window.dialing_directory_dialog.show_content(ctx, ui);
        });
    });

    if !open {
        window.show_terminal();
    }
}
