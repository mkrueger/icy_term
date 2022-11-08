use std::cmp::{max, min};

use iced::{Point, Rectangle, widget::canvas::{Geometry, Frame, Path, Stroke}, Color, Size};
use icy_engine::{Position, Buffer};

use super::calc;

#[derive(Debug, Clone)]
pub struct Selection {
    pub start: Point,
    pub block_selection: bool,

    pub selection_start:Position,
    pub selection_end:Position,
    pub selection_anchor_start:Position,
    pub selection_anchor_end:Position
}

impl Default for Selection {
    fn default() -> Self {
        Selection::new(Point::default())
    }
}

impl Selection {
    pub fn new(pos: Point) -> Self {
        Self {
            start: pos,
            block_selection: false,
            selection_start: Position::new(),
            selection_end: Position::new(),
            selection_anchor_start: Position::new(),
            selection_anchor_end: Position::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.selection_anchor_start == self.selection_anchor_end
    }
} 

impl Selection {
    pub fn update(&mut self, buffer: &Buffer, bounds: &Rectangle, cursor_pos: Point) {
        let (top_x, top_y, _, _, char_size) = calc(buffer, &bounds);
        let start = self.start;
        let end  = cursor_pos;
        self.selection_anchor_start = Position::from(
            ((start.x - top_x) / char_size.width.floor()) as i32,
            ((start.y - top_y) / char_size.height.floor()) as i32
        );
        self.selection_anchor_end = Position::from(
            ((end.x - top_x) / char_size.width.floor()) as i32,
            ((end.y - top_y) / char_size.height.floor()) as i32
        );
        self.selection_start = Position::from(min(self.selection_anchor_start.x, self.selection_anchor_end.x), min(self.selection_anchor_start.y, self.selection_anchor_end.y));
        self.selection_end = Position::from(max(self.selection_anchor_start.x, self.selection_anchor_end.x), max(self.selection_anchor_start.y, self.selection_anchor_end.y));
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

        if self.block_selection || self.selection_start.y == self.selection_end.y {
            let line = Path::rectangle(
                Point { 
                    x: top_x + self.selection_start.x as f32 * char_size.width, 
                    y: top_y + (self.selection_start.y - top_line) as f32 * char_size.height }, 
                iced::Size {
                    width: (self.selection_end.x - self.selection_start.x + 1) as f32 * char_size.width,
                    height: (self.selection_end.y - self.selection_start.y + 1) as f32 * char_size.height}
            );
            frame.stroke(&line, create_stroke());
        } else {
            let a = 1;
            let b = 0;

            // top border
            let line = Path::line(
                Point { x: top_x, y: top_y + (self.selection_anchor_start.y - top_line + a) as f32 * char_size.height }, 
                Point { x: top_x + self.selection_anchor_start.x as f32 * char_size.width, y: top_y + (self.selection_anchor_start.y - top_line + a) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());

            let line = Path::line(
                Point { x: top_x + self.selection_anchor_start.x as f32 * char_size.width, y: top_y + (self.selection_anchor_start.y - top_line) as f32 * char_size.height }, 
                Point { x: top_x + self.selection_anchor_start.x as f32 * char_size.width, y: top_y + (self.selection_anchor_start.y - top_line  + 1) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());
            let line = Path::line(
                Point { x: top_x + self.selection_anchor_start.x as f32 * char_size.width, y: top_y + (self.selection_anchor_start.y - top_line + b) as f32 * char_size.height }, 
                Point { x: top_x + w, y: top_y + (self.selection_anchor_start.y  - top_line + b) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());

            // bottom border

            let line = Path::line(
                Point { x: top_x, y: top_y + (self.selection_anchor_end.y + a  - top_line ) as f32 * char_size.height }, 
                Point { x: top_x + self.selection_anchor_end.x as f32 * char_size.width, y: top_y + (self.selection_anchor_end.y - top_line  + a) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());

            let line = Path::line(
                Point { x: top_x + self.selection_anchor_end.x as f32 * char_size.width, y: top_y + (self.selection_anchor_end.y - top_line  + 1)  as f32 * char_size.height }, 
                Point { x: top_x + self.selection_anchor_end.x as f32 * char_size.width, y: top_y + (self.selection_anchor_end.y - top_line) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());
            let line = Path::line(
                Point { x: top_x + self.selection_anchor_end.x as f32 * char_size.width, y: top_y + (self.selection_anchor_end.y - top_line + b) as f32 * char_size.height }, 
                Point { x: top_x + w, y: top_y + (self.selection_anchor_end.y - top_line + b) as f32 * char_size.height }, 
            );
            frame.stroke(&line, create_stroke());
        }
        frame.into_geometry()
    }
}

fn create_stroke<'a>() -> Stroke<'a>
{
    Stroke::default().with_width(2.0).with_color(Color::from_rgb8(21, 42, 253))
}