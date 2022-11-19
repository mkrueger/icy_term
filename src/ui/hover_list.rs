use iced::widget::canvas::{self, Cursor, Frame, Geometry, Text, Event, event, Path};
use iced::{ mouse, Point, Rectangle, Size, Theme, Color, Vector};

use crate::address::Address;

#[derive(Debug, Clone, Copy)]
pub enum CellState {
    Unselected,
    Hovered(bool),
    Selected
}

pub trait HoverListCell {
    fn draw_cell(&self, frame: &mut Frame, cell_rect: Rectangle, state: CellState);
    fn get_size(&self) -> Size;
}

pub struct HoverList {
    cache: canvas::Cache,
    cells: Vec<Box<dyn HoverListCell>>,
    pub selected_item: i32,
    pub spacing: f32
}

impl HoverListCell for Text {
    fn draw_cell(&self, frame: &mut Frame, cell_rect: Rectangle, state: CellState) {
        let mut t = self.clone();
        t.position = cell_rect.position();
        frame.fill_text(t);
    }

    fn get_size(&self) -> Size {
        Size::new(100.0, self.size)
    }
}

impl HoverListCell for Address {
    fn draw_cell(&self, frame: &mut Frame, cell_rect: Rectangle, state: CellState) {
        match state { 
             CellState::Hovered(_) => {
                let mut t = Text {
                    content: "\u{F54D}".to_string(),
                    color: Color::WHITE,
                    size: 22.0,
                    font: iced_aw::ICON_FONT,
                    ..Default::default()
                };
                t.position = cell_rect.position() + Vector::new(0.0, 0.0);
                frame.fill_text(t);
            }
            _  => {}
        }

        let mut t = if self.system_name.len() > 0 {  Text {
            content: self.system_name.clone(),
            color: Color::WHITE,
            size: 22.0,
            ..Default::default()
        } } else { 
            Text {
                content: "no name".to_string(),
                color: Color::from_rgb8(0xBB, 0xBB, 0xBB),
                size: 22.0,
                ..Default::default()
            }
        };
        t.position = cell_rect.position() + Vector::new(30.0, 0.0);
        frame.fill_text(t);
    }

    fn get_size(&self) -> Size {
        Size::new(100.0, 22.0)
    }
}


impl HoverList {
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
            cells: Vec::new(),
            spacing: 4.0,
            selected_item: -1
        }
    }

    pub fn get_height(&self) -> u16 {
        let res = self.cells.iter().map(|c| c.get_size().height).sum::<f32>() + self.spacing * self.cells.len() as f32;
        res as u16
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn add(&mut self, cell: Box<dyn HoverListCell>) {
        self.cells.push(cell);
    }

    pub fn update(&mut self) {
        self.cache.clear();
    }
}

#[derive(Default)]
pub struct DrawInfoState {
    hovered_item: i32,
}

#[derive(Debug, Clone)]
pub enum HoverListMessage {
    UpdateList,
    Selected(i32),
    CallBBS(i32)
}

impl<'a> canvas::Program<HoverListMessage> for HoverList {
    type State = DrawInfoState;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<HoverListMessage>) {
        let mut hovered_item = -1;
        if let Some(pos) = cursor.position() {
            if !bounds.contains(pos) {
                return (event::Status::Ignored, None);
            }

            let mut p = Point::new(0.0, 0.0);
            for i in 1..self.cells.len() {
                let cell = &self.cells[i];
                let size = Size { width: bounds.width, height: cell.get_size().height };
                let pos = Point::new(pos.x, pos.y - bounds.y);
                if Rectangle::new(p, size).contains(pos) {
                    hovered_item = i as i32;
                    break;
                }
                p.y += size.height + self.spacing;
            }
        }

        match event {
            Event::Mouse(mouse_event) => {
                match mouse_event {
                    mouse::Event::ButtonPressed(b) =>  {
                        if let Some(pos) = cursor.position() {
                            state.hovered_item = hovered_item;
                            if pos.x < 30.0 && hovered_item > 0 {
                                return (event::Status::Captured, Some(HoverListMessage::CallBBS(hovered_item)));
                            }
                        } else {
                            return (event::Status::Ignored, None);
                        }
                        return (event::Status::Captured, Some(HoverListMessage::Selected(hovered_item)));
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        if hovered_item != state.hovered_item {
            state.hovered_item = hovered_item;
            return (event::Status::Captured, Some(HoverListMessage::UpdateList));
        }

        (event::Status::Ignored, None)
    }

    fn draw(
        &self,
        state: &Self::State,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
     
        let content = self.cache.draw(bounds.size(), |frame: &mut Frame| {
            let background = canvas::Path::rectangle(Point::ORIGIN, frame.size());
            frame.fill(&background, iced::Color::from_rgb8(0x20, 0x22, 0x25));

            let mut p = Point::new(bounds.x, 0.0);
            for i in 1..self.cells.len() {
                let cell = &self.cells[i];
                let size = cell.get_size();
                let mut cell_state = CellState::Unselected;
                let back_p = Point::new(bounds.x + 25.0, p.y);

                if i as i32 == self.selected_item {
                    frame.fill_rectangle(back_p, Size::new(bounds.width - 20.0, size.height), Color::from_rgb8(0, 0x88, 0xE4));
                    if  self.selected_item == state.hovered_item {
                        cell_state = CellState::Hovered(true);
                    } else {
                        cell_state = CellState::Selected;
                    }
                } else if i as i32 == state.hovered_item {
                    frame.fill_rectangle(back_p, Size::new(bounds.width - 20.0, size.height), Color::from_rgb8(0, 0x38, 0x64));
                    cell_state = CellState::Hovered(false);
                }

                cell.draw_cell(frame, Rectangle::new(p, size), cell_state);
                p.y += size.height + self.spacing;
            }
        });

        let mut result = Vec::new();
        result.push(content);
        result
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(&bounds) && state.hovered_item >= 0 {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}
