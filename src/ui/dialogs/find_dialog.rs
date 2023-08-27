use egui::{FontFamily, FontId, Rect, RichText, SelectableLabel, TextEdit, Ui, Vec2};
use i18n_embed_fl::fl;
use icy_engine::{Buffer, Position, Selection, BufferParser, AttributedChar};
use icy_engine_egui::BufferView;

#[derive(Default)]
pub struct DialogState {
    pub pattern: Vec<char>,
    conv_pattern: Vec<char>,

    pub case_sensitive: bool,

    cur_sel: usize,
    cur_pos: Position,
    results: Vec<Position>,
}

pub enum Message {
    ChangePattern(String),
    FindNext,
    FindPrev,
    CloseDialog,
    SetCasing(bool),
}

fn set_lead(sel: &mut Selection, pos: Position, len: usize) {
    sel.set_lead(pos.x as f32 + len as f32, pos.y as f32);
}

impl DialogState {
    pub fn search_pattern(&mut self, buf: &Buffer, buffer_parser: &dyn BufferParser) {
        let mut cur_len = 0;
        let mut start_pos = Position::default();
        self.results.clear();
        if self.pattern.is_empty() {
            return;
        }

        if self.case_sensitive {
            self.conv_pattern = self.pattern.clone();
        } else { 
            self.conv_pattern = self.pattern.iter().map(char::to_ascii_lowercase).collect();
        }
        for y in 0..buf.get_real_buffer_height() {
            for x in 0..buf.get_buffer_width() {
                let ch = buf.get_char_xy(x, y);
                if self.compare(buffer_parser, cur_len, ch) {
                    if cur_len == 0 {
                        start_pos = Position::new(x, y);
                    }
                    cur_len += 1;
                    if cur_len >= self.pattern.len() {
                        self.results.push(start_pos);
                        cur_len = 0;
                    }
                } else if self.compare(buffer_parser, 0, ch) {
                    start_pos = Position::new(x, y);
                    cur_len = 1;
                } else {
                    cur_len = 0;
                }
            }
        }
    }

    fn compare(&mut self, buffer_parser: &dyn BufferParser, cur_len: usize, attributed_char: AttributedChar) -> bool {
        if self.case_sensitive {
            return self.conv_pattern[cur_len] == buffer_parser.convert_to_unicode(attributed_char);
        }
        self.conv_pattern[cur_len] == buffer_parser.convert_to_unicode(attributed_char).to_ascii_lowercase()
    }

    pub(crate) fn find_next(&mut self, buf: &mut BufferView) {
        if self.results.is_empty() || self.pattern.is_empty() {
            return;
        }
        self.cur_pos.x += 1;
        for (i, pos) in self.results.iter().enumerate() {
            if pos >= &self.cur_pos {
                let mut sel = Selection::new(pos.x as f32, pos.y as f32);
                set_lead(&mut sel, *pos, self.pattern.len());
                buf.set_selection(sel);
                self.cur_pos = *pos;
                self.cur_sel = i;
                return;
            }
        }
        self.cur_pos = Position::new(-1, -1);
        self.find_next(buf);
    }

    pub(crate) fn update_pattern(&mut self, buf: &mut BufferView) {
        if let Some(sel) = buf.get_selection() {
            if self.results.contains(&sel.anchor_pos) {
                let p = sel.anchor_pos;
                set_lead(sel, p, self.pattern.len());
                return;
            }
        }
        buf.clear_selection();
        self.find_next(buf);
    }

    pub(crate) fn find_prev(&mut self, buf: &mut BufferView) {
        if self.results.is_empty() || self.pattern.is_empty() {
            return;
        }
        self.cur_pos.x -= 1;
        let mut i = self.results.len() as i32 - 1;

        for pos in self.results.iter().rev() {
            if pos < &self.cur_pos {
                let mut sel = Selection::new(pos.x as f32, pos.y as f32);
                set_lead(&mut sel, *pos, self.pattern.len());
                buf.set_selection(sel);
                self.cur_pos = *pos;
                self.cur_sel = i as usize;
                return;
            }
            i -= 1;
        }
        self.cur_pos = Position::new(i32::MAX, i32::MAX);
        self.find_prev(buf);
    }

    pub fn show_ui(&self, ui: &mut Ui, rect: Rect) -> Option<Message> {
        let mut message = None;

        let mut pattern: String = self.pattern.iter().collect();
        let find_dialog_width: f32 = 400.0;
        let find_dialog_height: f32 = 64.0;
        let max_rect = Rect::from_min_size(
            rect.right_top() - Vec2::new(find_dialog_width, -2.0),
            Vec2::new(find_dialog_width - 8.0, find_dialog_height),
        );
        let img_size = 18.0;

        ui.allocate_ui_at_rect(max_rect, |ui| {
            ui.painter().rect(
                max_rect,
                4.0,
                ui.visuals().extreme_bg_color,
                ui.visuals().window_stroke,
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                let r = ui.add(
                    TextEdit::singleline(&mut pattern)
                        .hint_text(fl!(crate::LANGUAGE_LOADER, "terminal-find-hint")),
                );

                ui.memory_mut(|m| m.request_focus(r.id));

                if r.changed() {
                    message = Some(Message::ChangePattern(pattern));
                }

                let r = ui.add(SelectableLabel::new(
                    false,
                    RichText::new("⬆").font(FontId::new(img_size, FontFamily::Proportional)),
                ));
                if r.clicked() {
                    message = Some(Message::FindPrev);
                }

                let r = ui.add(SelectableLabel::new(
                    false,
                    RichText::new("⬇").font(FontId::new(img_size, FontFamily::Proportional)),
                ));
                if r.clicked() {
                    message = Some(Message::FindNext);
                }
                let r = ui.add(SelectableLabel::new(
                    false,
                    RichText::new("🗙").font(FontId::new(img_size, FontFamily::Proportional)),
                ));
                if r.clicked() {
                    message = Some(Message::CloseDialog);
                }

                if ui.input(|i| i.key_pressed(egui::Key::PageUp)) {
                    message = Some(Message::FindPrev);
                }
                if ui.input(|i| {
                    i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::PageDown)
                }) {
                    message = Some(Message::FindNext);
                }

                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    message = Some(Message::CloseDialog);
                }
            });
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                let r = ui.add(SelectableLabel::new(
                    self.case_sensitive,
                    RichText::new("🗛").font(FontId::new(img_size, FontFamily::Proportional)),
                ));
                if r.clicked() {
                    message = Some(Message::SetCasing(!self.case_sensitive));
                }

                if self.results.is_empty() {
                    if !self.pattern.is_empty() {
                        ui.colored_label(
                            ui.style().visuals.error_fg_color,
                            fl!(crate::LANGUAGE_LOADER, "terminal-find-no-results"),
                        );
                    }
                } else {
                    ui.label(fl!(
                        crate::LANGUAGE_LOADER,
                        "terminal-find-results",
                        cur = (self.cur_sel + 1).to_string(),
                        total = self.results.len().to_string()
                    ));
                }
            });
        });

        message
    }
}
