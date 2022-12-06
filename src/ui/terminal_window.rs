use std::{cmp::max};

use clipboard::{ClipboardProvider, ClipboardContext};
use eframe::{
    egui::{self, ScrollArea, CursorIcon, PointerButton, RichText},
    epaint::{Color32, Rect, Vec2, FontId},
};

use super::{main_window::{MainWindow, MainWindowMode}};

impl MainWindow {
    pub fn update_terminal_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let button_frame = egui::containers::Frame::none()
         .fill(Color32::from_rgb(0x20, 0x22, 0x25))
        .inner_margin(egui::style::Margin::same(8.0));
        let top_margin_height = 42.;
        egui::TopBottomPanel::top("button_bar")
            .frame(button_frame)
            .show(ctx, |ui| {
                let img_size = Vec2::new(22., 22.);

                ui.horizontal(|ui| {
                    if ui
                        .add(egui::ImageButton::new(
                            super::UPLOAD_SVG.texture_id(ctx),
                            img_size,
                        ))
                        .clicked()
                    {
                        self.mode = MainWindowMode::SelectProtocol(false);
                    }
                    if ui
                        .add(egui::ImageButton::new(
                            super::DOWNLOAD_SVG.texture_id(ctx),
                            img_size,
                        ))
                        .clicked()
                    {
                        self.mode = MainWindowMode::SelectProtocol(true);
                    }

                    if !self.auto_login.logged_in {
                        if ui
                            .add(egui::ImageButton::new(
                                super::KEY_SVG.texture_id(ctx),
                                img_size,
                            ))
                            .clicked()
                        {
                            self.send_login();
                        }
                    }
                    if ui
                        .add(egui::ImageButton::new(
                            super::CALL_END_SVG.texture_id(ctx),
                            img_size,
                        ))
                        .clicked()
                    {
                        self.hangup();
                    }


                    let text_style = FontId::proportional(22.);
                    let mut b = self.buffer_view.lock().crt_effect;
                    ui.checkbox( &mut b, RichText::new("CRT effect").font(text_style.clone()));
                    self.buffer_view.lock().crt_effect = b;
                });
            });

        let frame_no_margins = egui::containers::Frame::none()
            .inner_margin(egui::style::Margin::same(0.0))
            .fill(Color32::from_rgb(0x40, 0x44, 0x4b));
        egui::CentralPanel::default()
            .frame(frame_no_margins)
            .show(ctx, |ui| {
                self.custom_painting(ui, top_margin_height)
            });

    }

    fn custom_painting(&mut self, ui: &mut egui::Ui, top_margin_height: f32) -> egui::Response {
        let size = ui.available_size();
        let buffer_view = self.buffer_view.clone();
        let buf_w = buffer_view.lock().buf.get_buffer_width();
        let buf_h = buffer_view.lock().buf.get_buffer_height();
        // let h = max(buf_h, buffer_view.lock().buf.get_real_buffer_height());

        let font_dimensions = buffer_view.lock().buf.get_font_dimensions();

        let mut scale_x = (size.x - 4.0) / font_dimensions.width as f32 / buf_w as f32;
        let mut scale_y = size.y / font_dimensions.height as f32 / buf_h as f32;

        if scale_x < scale_y {
            scale_y = scale_x;
        } else {
            scale_x = scale_y;
        }

        let char_size = Vec2::new(font_dimensions.width as f32 * scale_x, font_dimensions.height as f32 * scale_y);

        let rect_w = buf_w as f32 * char_size.x;
        let rect_h = buf_h as f32 * char_size.y;
       
        let output = ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(true)
        .show_viewport(ui, |ui, viewport| {
            let (draw_area, mut response) = ui.allocate_at_least(size, egui::Sense::click());

            let rect = Rect::from_min_size(
                draw_area.left_top()
                    + Vec2::new(
                        (-4.0 + draw_area.width() - rect_w) / 2.,
                        (-top_margin_height + viewport.top() + (draw_area.height() - rect_h) / 2.).floor(),
                    )
                    .ceil(),
                Vec2::new(rect_w, rect_h),
            );
            let real_height = buffer_view.lock().buf.get_real_buffer_height();
            let max_lines = max(0, real_height - buf_h);
            ui.set_height(scale_y * max_lines as f32 * font_dimensions.height as f32);

            let first_line = (viewport.top() / char_size.y)  as i32;
            let scroll_back_line = max(0,  max_lines - first_line);

            if scroll_back_line != buffer_view.lock().scroll_back_line {
                buffer_view.lock().scroll_back_line = scroll_back_line;
                buffer_view.lock().redraw_view();
            }
            let callback = egui::PaintCallback {
                rect,
                callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                    buffer_view.lock().update_buffer(painter.gl());
                    buffer_view.lock().paint(painter.gl(), rect);
                })),
            };
            ui.painter().add(callback);
            response = response.context_menu(terminal_context_menu);
            let events = ui.input().events.clone();
            for e in &events {
                match e {
                    egui::Event::PointerButton {button: PointerButton::Middle, pressed: true, .. } | 
                    egui::Event::Copy => {
                        let buffer_view = self.buffer_view.clone();
                        let mut l = buffer_view.lock();
                        if let Some(txt) = l.get_copy_text(&self.buffer_parser) {
                            ui.output().copied_text = txt;
                        }
                    }
                    egui::Event::Cut => {}
                    egui::Event::Paste(text) => {
                        self.output_string(text);
                    }
                    egui::Event::CompositionEnd(text) | egui::Event::Text(text) => {
                        for c in text.chars() {
                            self.output_char(c);
                        }
                        response.mark_changed();
                    }

                    egui::Event::PointerButton { pos, button: PointerButton::Primary, pressed: true, modifiers } => {
                        if rect.contains(*pos) {
                            let buffer_view = self.buffer_view.clone();
                            let click_pos = (*pos - rect.min - Vec2::new(0., top_margin_height)) / char_size + Vec2::new(0.0, first_line as f32);
                            buffer_view.lock().selection_opt = Some(crate::ui::Selection::new(click_pos));
                            buffer_view.lock().selection_opt.as_mut().unwrap().block_selection = modifiers.alt;
                        }
                    }
                    
                    egui::Event::PointerButton {button: PointerButton::Primary, pressed: false, .. } => {
                        let buffer_view = self.buffer_view.clone();
                        let mut l = buffer_view.lock();
                        if let Some(sel) = &mut l.selection_opt {
                            sel.locked = true;
                        }
                    }
                    
                    egui::Event::PointerMoved(pos) => {
                        let buffer_view = self.buffer_view.clone();
                        let mut l = buffer_view.lock();
                        if let Some(sel) = &mut l.selection_opt {
                            if !sel.locked {
                                let click_pos = (*pos - rect.min - Vec2::new(0., top_margin_height)) / char_size + Vec2::new(0.0, first_line as f32);
                                sel.set_lead(click_pos);
                                sel.block_selection = ui.input().modifiers.alt;
                                l.redraw_view();
                            }
                        }
                    }
                    egui::Event::KeyRepeat {key, modifiers} |
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                    } => {
                        let im = self.screen_mode.get_input_mode();
                        let key_map = im.cur_map();
                        let mut key_code = *key as u32;
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
                                            eprintln!("{}", err);
                                        }
                                    }
                                }
                                response.mark_changed();
                                ui.input_mut().consume_key(*modifiers, *key);
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if response.hovered() {
                let hover_pos_opt = ui.input().pointer.hover_pos();
                if let Some(hover_pos) = hover_pos_opt { 
                    if rect.contains(hover_pos) {
                        ui.output().cursor_icon = CursorIcon::Text;
                    }
                }
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
    ui.input_mut().events.clear();

    if ui.button("Copy").clicked() {
        ui.input_mut().events.push(egui::Event::Copy);
        ui.close_menu();
    }

    if ui.button("Paste").clicked() {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Ok(text) = ctx.get_contents() {
            ui.input_mut().events.push(egui::Event::Paste(text));
        }
        ui.close_menu();
    }
}