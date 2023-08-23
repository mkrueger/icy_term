use egui::{Color32, Id, Pos2, Rect, Response, Sense, Ui, Vec2};

pub struct SmoothScroll {
    scroll_positon: f32,
    last_height: f32,
    id: Id,
}
impl SmoothScroll {
    pub fn new() -> Self {
        Self {
            scroll_positon: 0.0,
            last_height: 0.0,
            id: Id::new("smooth_scroll"),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui, Rect, &mut Response, f32) -> f32,
    ) -> Response {
        if let Some((scroll, last_height)) = ui
            .ctx()
            .memory_mut(|mem| mem.data.get_persisted::<(f32, f32)>(self.id))
        {
            self.scroll_positon = scroll;
            self.last_height = last_height;
        }
        let size = ui.available_size();

        let (id, rect) = ui.allocate_space(Vec2::new(size.x, size.y));
        let mut response = ui.interact(rect, id, Sense::click_and_drag());

        let scroll_height = add_contents(ui, rect, &mut response, self.scroll_positon);
        if (scroll_height - self.last_height).abs() > 0.1 {
            self.scroll_positon = (scroll_height - rect.height()).max(0.0);
        }

        if rect.height() > 0.0 && rect.width() > 0.0 {
            self.show_scrollbar(ui, scroll_height, &response, rect);
        }
        response
    }

    fn show_scrollbar(&mut self, ui: &Ui, scroll_height: f32, response: &Response, rect: Rect) {
        let scrollbar_width = ui.style().spacing.scroll_bar_width;

        let x = rect.right() - scrollbar_width;
        let mut bg_rect: Rect = rect;
        bg_rect.set_left(x);
        let bar_top = self.scroll_positon / scroll_height.max(1.0) * bg_rect.height();

        let bar_height = (bg_rect.height() * bg_rect.height() / scroll_height.max(1.0))
            .clamp(1.0, bg_rect.height());

        if (bar_height - bg_rect.height()).abs() < 1.0 {
            ui.ctx().memory_mut(|mem: &mut egui::Memory| {
                mem.data
                    .insert_persisted(self.id, (self.scroll_positon, scroll_height));
            });

            return;
        }

        let bar_offset = -bar_height / 2.0;

        let events: Vec<egui::Event> = ui.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Scroll(vec) = e {
                self.scroll_positon -= vec.y;
            }
        }

        if response.clicked() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                if mouse_pos.x > x {
                    let my = mouse_pos.y + bar_offset;
                    self.scroll_positon = scroll_height * (my - bg_rect.top()) / bg_rect.height();
                }
            }
        }

        let mut dragged = false;
        if response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                dragged = true;
                let my = mouse_pos.y + bar_offset;
                self.scroll_positon = scroll_height * (my - bg_rect.top()) / bg_rect.height();
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
        let how_on = ui.ctx().animate_bool(response.id, hovered || dragged);

        let x_size = egui::lerp(2.0..=scrollbar_width, how_on);
        self.scroll_positon = self
            .scroll_positon
            .clamp(0.0, (scroll_height - rect.height()).max(0.0));

        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.right() - x_size, bg_rect.top()),
                Vec2::new(x_size, rect.height()),
            ),
            0.,
            Color32::from_rgba_unmultiplied(0x3F, 0x3F, 0x3F, 32),
        );

        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.right() - x_size, bg_rect.top() + bar_top),
                Vec2::new(x_size, bar_height),
            ),
            4.,
            Color32::from_rgba_unmultiplied(0xFF, 0xFF, 0xFF, 0x5F + (127.0 * how_on) as u8),
        );

        ui.ctx().memory_mut(|mem: &mut egui::Memory| {
            mem.data
                .insert_persisted(self.id, (self.scroll_positon, scroll_height));
        });
    }
}
