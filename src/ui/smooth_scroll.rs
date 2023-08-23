use egui::{Color32, Id, Pos2, Rect, Response, Sense, Ui, Vec2};

pub struct SmoothScroll {
    /// Current scroll position in terminal pixels (not screen pixels)
    char_scroll_positon: f32,
    /// The height of the buffer in chars
    char_height: f32,
    /// The height of the visible area in chars
    buffer_height: f32,
    /// used to determine if the buffer should auto scroll to the bottom.
    last_char_height: f32,
    /// Height of a single terminal pixel in screen pixels
    scale: f32,
    font_height: f32,
    drag_start: bool,
    id: Id,
    lock_focus: bool,
}

impl SmoothScroll {
    pub fn new() -> Self {
        Self {
            id: Id::new("smooth_scroll"),
            char_scroll_positon: 0.0,
            last_char_height: 0.0,
            char_height: 0.0,
            scale: 0.0,
            buffer_height: 0.0,
            font_height: 0.0,
            drag_start: false,
            lock_focus: true,
        }
    }

    pub fn with_lock_focus(mut self, lock_focus: bool) -> Self {
        self.lock_focus = lock_focus;
        self
    }

    fn persist_data(&mut self, ui: &Ui) {
        ui.ctx().memory_mut(|mem: &mut egui::Memory| {
            mem.data.insert_persisted(
                self.id,
                (
                    self.char_scroll_positon,
                    self.char_height,
                    self.buffer_height,
                    self.last_char_height,
                    self.scale,
                    self.drag_start,
                ),
            );
        });
    }

    fn load_data(&mut self, ui: &Ui) {
        if let Some(scroll) = ui.ctx().memory_mut(|mem| {
            mem.data
                .get_persisted::<(f32, f32, f32, f32, f32, bool)>(self.id)
        }) {
            self.char_scroll_positon = scroll.0;
            self.char_height = scroll.1;
            self.buffer_height = scroll.2;
            self.last_char_height = scroll.3;
            self.scale = scroll.4;
            self.drag_start = scroll.5;
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        calc_contents: impl FnOnce(Rect) -> (f32, f32, f32, f32),
        add_contents: impl FnOnce(&mut Ui, Rect, Response, f32, Rect) -> Response,
    ) -> Response {
        self.load_data(ui);
        let size = ui.available_size();

        let (_, rect) = ui.allocate_space(Vec2::new(size.x, size.y));
        let response = ui.interact(rect, self.id, Sense::click_and_drag());

        if self.lock_focus {
            self.lock_focus = false;
            ui.memory_mut(|m| {
                m.request_focus(self.id);
                m.lock_focus(self.id, true);
            });
        }

        let (char_height, buffer_char_height, scale, font_height) = calc_contents(rect);

        self.char_height = char_height;
        self.buffer_height = buffer_char_height;
        self.scale = scale;
        self.font_height = font_height;

        if (char_height - self.last_char_height).abs() > 0.1 {
            self.char_scroll_positon = font_height * (char_height - buffer_char_height).max(0.0);
        }
        self.last_char_height = char_height;

        self.char_scroll_positon = self.char_scroll_positon.clamp(
            0.0,
            font_height * (self.char_height - self.buffer_height).max(0.0),
        );

        let scrollbar_width = ui.style().spacing.scroll_bar_width;
        let x = rect.right() - scrollbar_width;
        let mut scrollbar_rect: Rect = rect;
        scrollbar_rect.set_left(x);

        let response = add_contents(ui, rect, response, self.char_scroll_positon, scrollbar_rect);

        if char_height > buffer_char_height {
            self.show_scrollbar(ui, &response, rect);
        }

        self.persist_data(ui);

        response
    }

    fn show_scrollbar(&mut self, ui: &Ui, response: &Response, rect: Rect) {
        let scrollbar_width = ui.style().spacing.scroll_bar_width;

        let x = rect.right() - scrollbar_width;
        let mut bg_rect: Rect = rect;
        bg_rect.set_left(x);
        let bar_top = rect.top()
            + rect.height() * self.char_scroll_positon
                / (self.font_height * self.char_height.max(1.0));

        let bar_height = (self.buffer_height / self.char_height.max(1.0)) * rect.height();

        let bar_offset = -bar_height / 2.0;

        let events: Vec<egui::Event> = ui.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Scroll(vec) = e {
                self.char_scroll_positon -= vec.y;
            }
        }

        if response.clicked() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                if mouse_pos.x > x {
                    let my = mouse_pos.y + bar_offset;
                    self.char_scroll_positon =
                        self.char_height * self.font_height * (my - bg_rect.top())
                            / bg_rect.height();
                }
            }
        }

        let mut dragged: bool = false;

        if self.drag_start && response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                dragged = true;
                let my = mouse_pos.y + bar_offset;
                self.char_scroll_positon =
                    self.char_height * self.font_height * (my - bg_rect.top()) / bg_rect.height();
            }
        }
        let mut hovered = false;
        if response.hovered() {
            if let Some(mouse_pos) = response.hover_pos() {
                if mouse_pos.x > x {
                    hovered = true;
                }
            }
        }

        if hovered && response.drag_started() {
            self.drag_start = true;
        }

        if response.drag_released() {
            self.drag_start = false;
        }

        let how_on = ui.ctx().animate_bool(response.id, hovered || dragged);

        let x_size = egui::lerp(2.0..=scrollbar_width, how_on);

        // draw bg
        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.right() - x_size, bg_rect.top()),
                Vec2::new(x_size, rect.height()),
            ),
            0.,
            Color32::from_rgba_unmultiplied(0x3F, 0x3F, 0x3F, 32),
        );

        // draw bar
        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.right() - x_size, bar_top),
                Vec2::new(x_size, bar_height),
            ),
            4.,
            Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 0x5F + (127.0 * how_on) as u8),
        );
    }
}
