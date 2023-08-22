use egui::{Color32, Id, Pos2, Rect, Response, Sense, Ui, Vec2};

pub struct SmoothScroll {
    scroll_positon: f32,
    id: Id,
}
impl SmoothScroll {
    pub fn new() -> Self {
        Self {
            scroll_positon: 0.0,
            id: Id::new("smooth_scroll"),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui, Rect, &mut Response, f32) -> f32,
    ) -> Response {
        if let Some(scroll) = ui
            .ctx()
            .memory_mut(|mem| mem.data.get_persisted::<f32>(self.id))
        {
            self.scroll_positon = scroll;
        }
        let size = ui.available_size();

        let scrollbar_width = ui.style().spacing.scroll_bar_width;

        let (id, rect) = ui.allocate_space(Vec2::new(size.x, size.y));
        let mut response = ui.interact(rect, id, Sense::click_and_drag());

        let mut rect_corr = rect;
        rect_corr.set_width(rect.width() - scrollbar_width);

        let scroll_height = add_contents(ui, rect_corr, &mut response, self.scroll_positon);

        self.show_scrollbar(ui, scroll_height, &response, rect);
        response
    }

    fn show_scrollbar(&mut self, ui: &Ui, scroll_size: f32, response: &Response, rect: Rect) {
        let visuals = ui.style().interact_selectable(response, true);
        let scrollbar_width = ui.style().spacing.scroll_bar_width;

        let x = rect.right() - scrollbar_width;
        let mut bg_rect: Rect = rect;
        bg_rect.set_left(x);

        ui.painter().rect_filled(bg_rect, 0., Color32::GRAY);

        if response.clicked() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                if mouse_pos.x > x {
                    self.scroll_positon =
                        scroll_size * (mouse_pos.y - bg_rect.top()) / bg_rect.height();
                }
            }
        }
        if response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                self.scroll_positon =
                    scroll_size * (mouse_pos.y - bg_rect.top()) / bg_rect.height();
            }
        }
        self.scroll_positon = self
            .scroll_positon
            .clamp(0.0, (scroll_size - rect.height()).max(0.0));

        let h = (bg_rect.height() * bg_rect.height() / scroll_size.max(1.0))
            .clamp(1.0, bg_rect.height());

        ui.painter().rect_filled(
            Rect::from_min_size(
                Pos2::new(
                    x,
                    bg_rect.top() + self.scroll_positon / scroll_size.max(1.0) * bg_rect.height(),
                ),
                Vec2::new(scrollbar_width, h),
            ),
            0.5,
            Color32::WHITE,
        );

        ui.ctx().memory_mut(|mem: &mut egui::Memory| {
            mem.data.insert_persisted(self.id, self.scroll_positon);
        });
    }
}
