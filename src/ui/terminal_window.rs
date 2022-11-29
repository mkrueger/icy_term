use std::cmp::max;

use eframe::{egui, epaint::{Rect, Vec2, Color32}};

use super::main_window::{MainWindow, MainWindowMode};

impl MainWindow {
    pub fn update_terminal_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_no_margins = egui::containers::Frame::none().inner_margin(egui::style::Margin::same(0.0)).fill(Color32::from_rgb(0x40, 0x44, 0x4b));

        egui::TopBottomPanel::top("button_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Upload").clicked() {
                    self.mode = MainWindowMode::SelectProtocol(false);
                }
                if ui.button("Download").clicked() {
                    self.mode = MainWindowMode::SelectProtocol(true);
                }
                if ui.button("Login").clicked() { 
                    self.send_login();
                }
                if ui.button("Hangup").clicked() { 
                    self.hangup();
                }
            });
        });

        egui::CentralPanel::default()
            .frame(frame_no_margins)
            .show(ctx, |ui| {
                self.custom_painting(ui);
            });
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let buffer_view = self.buffer_view.clone();
        let buf_w = buffer_view.lock().buf.get_buffer_width();
        let buf_h = buffer_view.lock().buf.get_buffer_height();
        let h = max(buf_h, buffer_view.lock().buf.get_real_buffer_height());
        let (rect, _) = ui.allocate_at_least(size, egui::Sense::drag());

        let font_dimensions = buffer_view.lock().buf.get_font_dimensions();

        let mut scale_x = rect.width() / font_dimensions.width as f32 / buf_w as f32;
        let mut scale_y = rect.height() / font_dimensions.height as f32 / buf_h as f32;
    
        if scale_x < scale_y {
            scale_y = scale_x;
        } else {
            scale_x = scale_y;
        }

        let rect_w = scale_x * buf_w as f32 * font_dimensions.width as f32;
        let rect_h = scale_y * buf_h as f32 * font_dimensions.height as f32;

        let rect = Rect::from_min_size(rect.left_top() + Vec2::new((rect.width() - rect_w) / 2., (1. + rect.height() - rect_h) / 2.).ceil(), Vec2::new(rect_w, rect_h));
        

        let callback = egui::PaintCallback {
            rect: rect,
            callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                buffer_view.lock().update_buffer(painter.gl());
                buffer_view.lock().paint(painter.gl(), rect);
            })),
        };
        ui.painter()
        .add(callback);

        let events = ui.input().events.clone(); // avoid dead-lock by cloning. TODO(emilk): optimize
        for e in &events {
            match e {
                egui::Event::Copy => {},
                egui::Event::Cut => {},
                egui::Event::Paste(_) => {},
                egui::Event::Text(text) => {
                    for c in text.chars() {
                        self.output_char(c);
                    }
                },
                egui::Event::Scroll(x) => {
                    self.buffer_view.lock().scroll((x.y as i32) / 10);
                },
                egui::Event::Key { key, pressed, modifiers } => {
                    if *pressed {
                        let im = self.screen_mode.get_input_mode();
                        let key_map = im.cur_map();
                        let key = *key as u32;
                        
                        for (k, m) in key_map {
                            if *k == key {
                                self.handled_char = true;
                                for c in *m {
                                    self.output_char(unsafe { char::from_u32_unchecked(*c as u32)});
                                }
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

    }
}
