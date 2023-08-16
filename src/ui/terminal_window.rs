use std::cmp::max;

use clipboard::{ClipboardContext, ClipboardProvider};
use eframe::{
    egui::{self, CursorIcon, PointerButton, RichText, ScrollArea},
    epaint::{Color32, FontFamily, FontId, Rect, Vec2},
};
use i18n_embed_fl::fl;

use super::main_window_mod::{MainWindow, MainWindowMode};

fn encode_mouse_button(button: i32) -> char {
    unsafe { char::from_u32_unchecked(b' '.saturating_add(button as u8) as u32) }
}
fn encode_mouse_position(pos: i32) -> char {
    unsafe { char::from_u32_unchecked(b'!'.saturating_add(pos as u8) as u32) }
}

impl MainWindow {
    pub fn update_terminal_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let button_frame = egui::containers::Frame::none()
            .fill(Color32::from_rgb(0x20, 0x22, 0x25))
            .inner_margin(egui::style::Margin::same(6.0));
        let top_margin_height = 38.;
        let show_pb = matches!(self.mode, MainWindowMode::ShowPhonebook);
        egui::TopBottomPanel::top("button_bar")
            .frame(button_frame)
            .show(ctx, |ui| {
                let img_size = 20.0;
                if show_pb {
                    ui.set_enabled(false);
                }
                ui.horizontal(|ui| {
                    let r = ui
                        .button(
                            RichText::new("⬆")
                                .font(FontId::new(img_size, FontFamily::Proportional)),
                        )
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(crate::LANGUAGE_LOADER, "terminal-upload"))
                                    .small(),
                            );
                        });

                    if r.clicked() {
                        self.mode = MainWindowMode::SelectProtocol(false);
                    }

                    let r = ui
                        .button(
                            RichText::new("⬇")
                                .font(FontId::new(img_size, FontFamily::Proportional)),
                        )
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(crate::LANGUAGE_LOADER, "terminal-download"))
                                    .small(),
                            );
                        });

                    if r.clicked() {
                        self.mode = MainWindowMode::SelectProtocol(true);
                    }

                    if !self.auto_login.logged_in {
                        let r = ui
                            .button(
                                RichText::new("🔑")
                                    .font(FontId::new(img_size, FontFamily::Monospace)),
                            )
                            .on_hover_ui(|ui| {
                                ui.label(
                                    RichText::new(fl!(
                                        crate::LANGUAGE_LOADER,
                                        "terminal-autologin"
                                    ))
                                    .small(),
                                );
                            });

                        if r.clicked() {
                            self.send_login();
                        }
                    }

                    let r: egui::Response = ui
                        .add(egui::Button::new(
                            RichText::new("📞").font(FontId::new(img_size, FontFamily::Monospace)),
                        ))
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(crate::LANGUAGE_LOADER, "terminal-phonebook"))
                                    .small(),
                            );
                        });

                    if r.clicked() {
                        self.show_phonebook();
                    }

                    let size = ui.available_size_before_wrap();
                    ui.add_space(size.x - 70.0);

                    let r = ui
                        .button(
                            RichText::new("☎").font(FontId::new(img_size, FontFamily::Monospace)),
                        )
                        .on_hover_ui(|ui| {
                            ui.label(
                                RichText::new(fl!(crate::LANGUAGE_LOADER, "terminal-hangup"))
                                    .small(),
                            );
                        });
                    if r.clicked() {
                        self.hangup();
                    }

                    ui.menu_button(
                        RichText::new("☰")
                            .font(FontId::new(img_size + 6., FontFamily::Proportional)),
                        |ui| {
                            let r = ui.hyperlink_to(
                                fl!(crate::LANGUAGE_LOADER, "menu-item-discuss"),
                                "https://github.com/mkrueger/icy_term/discussions",
                            );
                            if r.clicked() {
                                ui.close_menu();
                            }
                            let r = ui.hyperlink_to(
                                fl!(crate::LANGUAGE_LOADER, "menu-item-report-bug"),
                                "https://github.com/mkrueger/icy_term/issues/new",
                            );
                            if r.clicked() {
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .button(fl!(crate::LANGUAGE_LOADER, "menu-item-settings"))
                                .clicked()
                            {
                                self.show_settings(false);
                                ui.close_menu();
                            }
                        },
                    );
                });
            });

        let frame_no_margins = egui::containers::Frame::none()
            .inner_margin(egui::style::Margin::same(0.0))
            .fill(Color32::from_rgb(0x40, 0x44, 0x4b));
        egui::CentralPanel::default()
            .frame(frame_no_margins)
            .show(ctx, |ui| {
                self.custom_painting(ui, top_margin_height);
            });

        if show_pb {
            super::view_phonebook(self, ctx);
        }
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui, top_margin_height: f32) -> egui::Response {
        let buf_h = self.buffer_view.lock().buf.get_buffer_height();
        let real_height = self.buffer_view.lock().buf.get_real_buffer_height();

        let output = ScrollArea::vertical()
            .auto_shrink([false; 2])
            /*.scroll_bar_visibility(if real_height <= buf_h {
                egui::scroll_area::ScrollBarVisibility::AlwaysHidden
            } else {
                egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded
            })*/
            .stick_to_bottom(true)
            .show_viewport(ui, |ui, viewport| {
                let (id, rect) =
                    ui.allocate_space(Vec2::new(ui.available_size().x, ui.available_size().y + 3.));
                let mut response = ui.interact(rect, id, egui::Sense::click());

                let size = rect.size();
                let buffer_view = self.buffer_view.clone();
                let buf_w = buffer_view.lock().buf.get_buffer_width();
                // let h = max(buf_h, buffer_view.lock().buf.get_real_buffer_height());

                let font_dimensions = buffer_view.lock().buf.get_font_dimensions();

                let mut scale_x = size.x / font_dimensions.width as f32 / buf_w as f32;
                let mut scale_y = size.y / font_dimensions.height as f32 / buf_h as f32;

                if scale_x < scale_y {
                    scale_y = scale_x;
                } else {
                    scale_x = scale_y;
                }

                let char_size = Vec2::new(
                    font_dimensions.width as f32 * scale_x,
                    font_dimensions.height as f32 * scale_y,
                );

                let rect_w = buf_w as f32 * char_size.x;
                let rect_h = buf_h as f32 * char_size.y;

                let terminal_rect = Rect::from_min_size(
                    rect.left_top()
                        + Vec2::new(
                            3. + (rect.width() - rect_w) / 2.,
                            (-top_margin_height + viewport.top() + (rect.height() - rect_h) / 2.)
                                .floor(),
                        )
                        .ceil(),
                    Vec2::new(rect_w, rect_h),
                );
                let buf_h = buffer_view.lock().buf.get_buffer_height();

                let max_lines = max(0, real_height - buf_h);

                // Set the scrolling height.
                ui.set_height(char_size.y * max_lines as f32);

                let first_line = (viewport.top() / char_size.y) as i32;

                {
                    buffer_view.lock().char_size = char_size;
                    if buffer_view.lock().viewport_top != viewport.top() {
                        buffer_view.lock().viewport_top = viewport.top();
                        buffer_view.lock().redraw_view();
                    }
                }

                let callback = egui::PaintCallback {
                    rect,
                    callback: std::sync::Arc::new(egui_glow::CallbackFn::new(
                        move |info, painter| {
                            buffer_view.lock().update_contents(painter.gl());
                            buffer_view
                                .lock()
                                .render_contents(painter.gl(), &info, terminal_rect);
                        },
                    )),
                };
                ui.painter().add(callback);

                // if self.buffer_view.lock().buf.terminal_state.mouse_mode
                //     != icy_engine::MouseMode::VT200
                {
                    response = response.context_menu(terminal_context_menu);
                }

                if matches!(self.mode, MainWindowMode::ShowTerminal) && ui.is_enabled() {
                    let events = ui.input(|i| i.events.clone());
                    for e in events {
                        match e {
                            egui::Event::PointerButton {
                                button: PointerButton::Middle,
                                pressed: true,
                                ..
                            }
                            | egui::Event::Copy => {
                                let buffer_view = self.buffer_view.clone();
                                let mut l = buffer_view.lock();
                                if let Some(txt) = l.get_copy_text(&*self.buffer_parser) {
                                    ui.output_mut(|o| o.copied_text = txt);
                                }
                            }
                            egui::Event::Paste(text) => {
                                self.output_string(&text);
                            }
                            egui::Event::CompositionEnd(text) | egui::Event::Text(text) => {
                                for c in text.chars() {
                                    self.output_char(c);
                                }
                                response.mark_changed();
                            }

                            egui::Event::PointerButton {
                                pos,
                                button,
                                pressed: true,
                                modifiers,
                            } => {
                                if terminal_rect.contains(pos) {
                                    let buffer_view = self.buffer_view.clone();
                                    let click_pos = (pos
                                        - terminal_rect.min
                                        - Vec2::new(0., top_margin_height))
                                        / char_size
                                        + Vec2::new(0.0, first_line as f32);

                                    let mode: icy_engine::MouseMode =
                                        buffer_view.lock().buf.terminal_state.mouse_mode;

                                    if matches!(button, PointerButton::Primary) {
                                        buffer_view
                                            .lock()
                                            .set_selection(crate::ui::Selection::new(click_pos));
                                        buffer_view
                                            .lock()
                                            .get_selection()
                                            .as_mut()
                                            .unwrap()
                                            .block_selection = modifiers.alt;
                                    }
                                    match mode {
                                        icy_engine::MouseMode::VT200
                                        | icy_engine::MouseMode::VT200_Highlight => {
                                            let mut modifier_mask = 0;
                                            if matches!(button, PointerButton::Secondary) {
                                                modifier_mask |= 1;
                                            }
                                            if modifiers.shift {
                                                modifier_mask |= 4;
                                            }
                                            if modifiers.alt {
                                                modifier_mask |= 8;
                                            }
                                            if modifiers.ctrl || modifiers.mac_cmd {
                                                modifier_mask |= 16;
                                            }
                                            self.output_string(
                                                format!(
                                                    "\x1b[M{}{}{}",
                                                    encode_mouse_button(modifier_mask),
                                                    encode_mouse_position(click_pos.x as i32),
                                                    encode_mouse_position(
                                                        click_pos.y as i32 - first_line
                                                    )
                                                )
                                                .as_str(),
                                            );
                                        }
                                        icy_engine::MouseMode::X10 => {
                                            self.output_string(
                                                format!(
                                                    "\x1b[M{}{}{}",
                                                    encode_mouse_button(0),
                                                    encode_mouse_position(click_pos.x as i32),
                                                    encode_mouse_position(click_pos.y as i32)
                                                )
                                                .as_str(),
                                            );
                                        }
                                        _ => {} /*
                                                icy_engine::MouseMode::ButtonEvents => todo!(),
                                                icy_engine::MouseMode::AnyEvents => todo!(),
                                                icy_engine::MouseMode::FocusEvent => todo!(),
                                                icy_engine::MouseMode::AlternateScroll => todo!(),
                                                icy_engine::MouseMode::ExtendedMode => todo!(),
                                                icy_engine::MouseMode::SGRExtendedMode => todo!(),
                                                icy_engine::MouseMode::URXVTExtendedMode => todo!(),
                                                icy_engine::MouseMode::PixelPosition => todo!(),*/
                                    }
                                }
                            }

                            egui::Event::PointerButton {
                                pos,
                                button: PointerButton::Primary,
                                pressed: false,
                                modifiers,
                                ..
                            } => {
                                if terminal_rect.contains(pos) {
                                    if let Some(sel) = self.buffer_view.lock().get_selection() {
                                        sel.locked = true;
                                    }
                                }

                                let mode: icy_engine::MouseMode =
                                    self.buffer_view.lock().buf.terminal_state.mouse_mode;
                                match mode {
                                    icy_engine::MouseMode::VT200
                                    | icy_engine::MouseMode::VT200_Highlight => {
                                        if terminal_rect.contains(pos) {
                                            let click_pos = (pos
                                                - terminal_rect.min
                                                - Vec2::new(0., top_margin_height))
                                                / char_size
                                                + Vec2::new(0.0, first_line as f32);

                                            let mut modifier_mask = 3; // 3 means realease
                                            if modifiers.shift {
                                                modifier_mask |= 4;
                                            }
                                            if modifiers.alt {
                                                modifier_mask |= 8;
                                            }
                                            if modifiers.ctrl || modifiers.mac_cmd {
                                                modifier_mask |= 16;
                                            }
                                            self.output_string(
                                                format!(
                                                    "\x1b[M{}{}{}",
                                                    encode_mouse_button(modifier_mask),
                                                    encode_mouse_position(click_pos.x as i32),
                                                    encode_mouse_position(click_pos.y as i32)
                                                )
                                                .as_str(),
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            egui::Event::PointerMoved(pos) => {
                                if terminal_rect.contains(pos) {
                                    let buffer_view = self.buffer_view.clone();
                                    let mut l = buffer_view.lock();
                                    if let Some(sel) = &mut l.get_selection() {
                                        if !sel.locked {
                                            let click_pos = (pos
                                                - terminal_rect.min
                                                - Vec2::new(0., top_margin_height))
                                                / char_size
                                                + Vec2::new(0.0, first_line as f32);
                                            sel.set_lead(click_pos);
                                            sel.block_selection = ui.input(|i| i.modifiers.alt);
                                            l.redraw_view();
                                        }
                                    }
                                }
                            }
                            egui::Event::Key {
                                key,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                let im = self.screen_mode.get_input_mode();
                                let key_map = im.cur_map();
                                let mut key_code = key as u32;
                                if modifiers.ctrl || modifiers.command {
                                    key_code |= super::CTRL_MOD;
                                }
                                if modifiers.shift {
                                    key_code |= super::SHIFT_MOD;
                                }
                                for (k, m) in key_map {
                                    if *k == key_code {
                                        self.handled_char = true;
                                        if let Some(con) = &mut self.connection_opt {
                                            let res = con.send(m.to_vec());
                                            self.handle_result(res, true);
                                        } else {
                                            for c in *m {
                                                if let Err(err) = self.print_char(*c) {
                                                    eprintln!("{err}");
                                                }
                                            }
                                        }
                                        response.mark_changed();
                                        ui.input_mut(|i| i.consume_key(modifiers, key));
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if response.hovered() {
                        let hover_pos_opt = ui.input(|i| i.pointer.hover_pos());
                        if let Some(hover_pos) = hover_pos_opt {
                            if terminal_rect.contains(hover_pos) {
                                ui.output_mut(|o| o.cursor_icon = CursorIcon::Text);
                            }
                        }
                    }
                } else {
                    self.buffer_view.lock().clear_selection();
                }
                response.dragged = false;
                response.drag_released = true;
                response.is_pointer_button_down_on = false;
                response.interact_pointer_pos = None;
                response
            });

        output.inner
    }
}

fn terminal_context_menu(ui: &mut egui::Ui) {
    ui.input_mut(|i| i.events.clear());

    if ui
        .button(fl!(crate::LANGUAGE_LOADER, "terminal-menu-copy"))
        .clicked()
    {
        ui.input_mut(|i| i.events.push(egui::Event::Copy));
        ui.close_menu();
    }

    if ui
        .button(fl!(crate::LANGUAGE_LOADER, "terminal-menu-paste"))
        .clicked()
    {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Ok(text) = ctx.get_contents() {
            ui.input_mut(|i| i.events.push(egui::Event::Paste(text)));
        }
        ui.close_menu();
    }
}
