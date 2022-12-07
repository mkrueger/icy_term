use eframe::epaint::Vec2;
use icy_engine::Position;

#[derive(Debug, Clone)]
pub struct Selection {
    pub anchor: Vec2,
    pub lead: Vec2,
    pub block_selection: bool,

    pub anchor_pos: Position,
    pub lead_pos: Position,

    pub locked: bool,
}

impl Default for Selection {
    fn default() -> Self {
        Selection::new(Vec2::default())
    }
}

impl Selection {
    pub fn new(pos: Vec2) -> Self {
        Self {
            anchor: pos,
            lead: pos,
            anchor_pos: Position::new(pos.x as i32, pos.y as i32),
            lead_pos: Position::new(pos.x as i32, pos.y as i32),
            block_selection: false,
            locked: false,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.anchor_pos == self.lead_pos
    }
}

impl Selection {
    pub fn set_lead(&mut self, lead: Vec2) {
        self.lead = lead;
        self.lead_pos = Position::new(lead.x as i32, lead.y as i32);
    }
}
