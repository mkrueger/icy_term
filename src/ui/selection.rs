use std::cmp::{max, min};

use iced::{Point, Rectangle, widget::canvas::{Geometry, Frame, Path, Stroke}, Color, Size};
use icy_engine::{Position, Buffer};

use super::calc;

#[derive(Debug, Clone)]
pub struct Selection {
    pub anchor_start_pt: Point,
    pub block_selection: bool,

    pub selection_start:Position,
    pub selection_end:Position,
    pub anchor :Position,
    pub lead :Position
}

impl Default for Selection {
    fn default() -> Self {
        Selection::new(Point::default())
    }
}

impl Selection {
    pub fn new(pos: Point) -> Self {
        Self {
            anchor_start_pt: pos,
            block_selection: false,
            selection_start: Position::new(),
            selection_end: Position::new(),
            anchor: Position::new(),
            lead: Position::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.anchor == self.lead
    }
} 

impl Selection {
    pub fn update(&mut self, buffer: &Buffer, bounds: &Rectangle, cursor_pos: Point) {
        let (top_x, top_y, _, _, char_size) = calc(buffer, &bounds);
        let start = self.anchor_start_pt;
        let end  = cursor_pos;
        self.anchor = Position::from(
            ((start.x - top_x) / char_size.width.floor()) as i32,
            ((start.y - top_y) / char_size.height.floor()) as i32
        );
        self.lead = Position::from(
            ((end.x - top_x) / char_size.width.floor()) as i32,
            ((end.y - top_y) / char_size.height.floor()) as i32
        );
        self.selection_start = Position::from(min(self.anchor.x, self.lead.x), min(self.anchor.y, self.lead.y));
        self.selection_end = Position::from(max(self.anchor.x, self.lead.x), max(self.anchor.y, self.lead.y));
    }

    pub fn draw(&self,buffer: &Buffer, scroll_back_line: i32, bounds: &Rectangle) -> Geometry {
        let mut frame = Frame::new(bounds.size());
        let (top_x, top_y, _, _, char_size) = calc(buffer, &bounds);

        let char_size = Size {
            width: char_size.width.floor(),
            height:char_size.height.floor(),
        };
        let w = buffer.get_buffer_width() as f32 * char_size.width;

        let top_line = buffer.get_first_visible_line() - scroll_back_line;
        let fill_color = Color::from_rgba8(0x24, 0xCC, 0xDD, 0.05);

        if self.block_selection || self.selection_start.y == self.selection_end.y {
            let line = Path::rectangle(
                Point { 
                    x: top_x + self.selection_start.x as f32 * char_size.width, 
                    y: top_y + (self.selection_start.y - top_line) as f32 * char_size.height }, 
                iced::Size {
                    width: (self.selection_end.x - self.selection_start.x + 1) as f32 * char_size.width,
                    height: (self.selection_end.y - self.selection_start.y + 1) as f32 * char_size.height}
            );
            frame.fill(&line, fill_color);
            frame.stroke(&line, create_stroke());
        } else {
            let a = 1;
            let b = 0;

            frame.fill_rectangle(
                Point { x: top_x, y: top_y + (self.selection_start.y  + 1 - top_line) as f32 * char_size.height }, 
                Size { width: w, height: (self.selection_end.y - self.selection_start.y - 1) as f32 * char_size.height },
                fill_color);
{
            let (a, b) = if self.anchor < self.lead { (self.anchor, self.lead) } else { (self.lead, self.anchor) };
            frame.fill_rectangle(
                Point { x: top_x + a.x as f32 * char_size.width, y: top_y + (a.y - top_line) as f32 * char_size.height }, 
                Size { width: w - (a.x as f32 * char_size.width), height: char_size.height },
                fill_color);
            
                frame.fill_rectangle(
                    Point { x: top_x, y: top_y + (b.y - top_line) as f32 * char_size.height }, 
                    Size { width: b.x as f32 * char_size.width, height: char_size.height },
                    fill_color);
                }

            // top border
            let line = Path::line(
                Point { x: top_x, y: top_y + (self.anchor.y - top_line + a) as f32 * char_size.height }, 
                Point { x: top_x + self.anchor.x as f32 * char_size.width, y: top_y + (self.anchor.y - top_line + a) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());

            let line = Path::line(
                Point { x: top_x + self.anchor.x as f32 * char_size.width, y: top_y + (self.anchor.y - top_line) as f32 * char_size.height }, 
                Point { x: top_x + self.anchor.x as f32 * char_size.width, y: top_y + (self.anchor.y - top_line  + 1) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());
            let line = Path::line(
                Point { x: top_x + self.anchor.x as f32 * char_size.width, y: top_y + (self.anchor.y - top_line + b) as f32 * char_size.height }, 
                Point { x: top_x + w, y: top_y + (self.anchor.y  - top_line + b) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());

            // bottom border

            let line = Path::line(
                Point { x: top_x, y: top_y + (self.lead.y + a - top_line) as f32 * char_size.height }, 
                Point { x: top_x + self.lead.x as f32 * char_size.width, y: top_y + (self.lead.y - top_line  + a) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());

            let line = Path::line(
                Point { x: top_x + self.lead.x as f32 * char_size.width, y: top_y + (self.lead.y - top_line  + 1)  as f32 * char_size.height }, 
                Point { x: top_x + self.lead.x as f32 * char_size.width, y: top_y + (self.lead.y - top_line) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());
            let line = Path::line(
                Point { x: top_x + self.lead.x as f32 * char_size.width, y: top_y + (self.lead.y - top_line + b) as f32 * char_size.height }, 
                Point { x: top_x + w, y: top_y + (self.lead.y - top_line + b) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());
        }
        frame.into_geometry()
    }
}

fn create_stroke<'a>() -> Stroke<'a>
{
    Stroke::default().with_width(2.0).with_color(Color::from_rgb8(0xAE, 0xAE, 0xAE))
}