use iced::Point;

pub struct Selection {
    pub start: Point,
    pub end: Point,
    pub block_selection: bool
}

impl Selection {
    pub fn new(pos: Point) -> Self {
        Self {
            start: pos,
            end: pos,
            block_selection: false
        }
    }
}