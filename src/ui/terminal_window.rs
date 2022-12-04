use std::cmp::max;

use crate::TerminalResult;
use eframe::{
    egui::{self, ScrollArea},
    epaint::{Color32, Rect, Vec2},
};

use super::main_window::{MainWindow, MainWindowMode};

impl MainWindow {
    pub fn update_terminal_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let button_frame = egui::containers::Frame::none()
            .fill(Color32::from_rgb(0x20, 0x22, 0x25))
            .inner_margin(egui::style::Margin::same(4.0));
        let top_margin_height = 40.;
        egui::TopBottomPanel::top("button_bar")
            .frame(button_frame)
            .show(ctx, |ui| {
                let img_size = Vec2::new(24., 24.);

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
                });
            });

        let frame_no_margins = egui::containers::Frame::none()
            .inner_margin(egui::style::Margin::same(0.0))
            .fill(Color32::from_rgba_unmultiplied(0x40, 0x44, 0x4b, 0xAF));
        egui::CentralPanel::default()
            .frame(frame_no_margins)
            .show(ctx, |ui| {
                let res = self.custom_painting(ui, top_margin_height);
                if let Err(err) = res {
                    eprintln!("{}", err);
                }
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui, top_margin_height: f32) -> TerminalResult<()> {
        let size = ui.available_size();
        let buffer_view = self.buffer_view.clone();
        let buf_w = buffer_view.lock().buf.get_buffer_width();
        let buf_h = buffer_view.lock().buf.get_buffer_height();
        // let h = max(buf_h, buffer_view.lock().buf.get_real_buffer_height());

        let font_dimensions = buffer_view.lock().buf.get_font_dimensions();

        let mut scale_x = (size.x + 5.0) / font_dimensions.width as f32 / buf_w as f32;
        let mut scale_y = size.y / font_dimensions.height as f32 / buf_h as f32;

        if scale_x < scale_y {
            scale_y = scale_x;
        } else {
            scale_x = scale_y;
        }
        let rect_w = scale_x * buf_w as f32 * font_dimensions.width as f32;
        let rect_h = scale_y * buf_h as f32 * font_dimensions.height as f32;
       
        ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(true)
        .show_viewport(ui, |ui, viewport| {
            let (rect, mut response) = ui.allocate_at_least(size, egui::Sense::drag());
            let rect = Rect::from_min_size(
                rect.left_top()
                    + Vec2::new(
                        (rect.width() - rect_w) / 2.,
                        (viewport.top() + (rect.height() - rect_h) / 2.).floor(),
                    )
                    .ceil(),
                Vec2::new(rect_w, rect_h),
            );
            let real_height = buffer_view.lock().buf.get_real_buffer_height();
            let max_lines = max(0, real_height - buf_h);
            ui.set_height(scale_y * max_lines as f32 * font_dimensions.height as f32);

            let scroll_back_line = max(0,  max_lines - (viewport.top() / scale_y / (font_dimensions.height as f32))  as i32);

            if scroll_back_line != buffer_view.lock().scroll_back_line {
                buffer_view.lock().scroll_back_line = scroll_back_line;
                buffer_view.lock().redraw_view();
            }
            let callback = egui::PaintCallback {
                rect: rect,
                callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                    buffer_view.lock().update_buffer(painter.gl());
                    buffer_view.lock().paint(painter.gl(), rect, top_margin_height);
                })),
            };
            ui.painter().add(callback);

            let events = ui.input().events.clone(); // avoid dead-lock by cloning. TODO(emilk): optimize
        for e in &events {
            match e {
                egui::Event::Copy => {}
                egui::Event::Cut => {}
                egui::Event::Paste(_) => {}
                egui::Event::CompositionEnd(text) | egui::Event::Text(text) => {
                    for c in text.chars() {
                        self.output_char(c);
                    }
                    response.mark_changed();
                }/* 
                egui::Event::Scroll(x) => {
                    self.buffer_view.lock().scroll((x.y as i32) / 10);
                }*/
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
                                con.send(m.to_vec());
                            } else {
                                for c in *m {
                                    self.print_char(*c);
                                }
                            }
                            response.mark_changed();
//                            ui.input_mut().consume_key(*modifiers, *key);
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        });

        
        Ok(())
    }
}
