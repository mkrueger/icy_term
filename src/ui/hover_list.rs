use crate::com::Com;
use clipboard::{ClipboardContext, ClipboardProvider};
use iced::keyboard::KeyCode;
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{self, Cursor, Frame, Geometry};
use iced::{keyboard, mouse, Point, Rectangle, Size, Theme};
use iced_graphics::Primitive;
use iced_native::image;
use icy_engine::{AvatarParser, Buffer, BufferParser, Caret, Position, SixelReadStatus};
use std::cmp::{max, min};

use super::Message;

pub struct HoverList {
    cache: canvas::Cache,
}

impl HoverList {
    pub fn new() -> Self {
        Self {
            cache: Default::default()
        }
    }
}

#[derive(Default)]
pub struct DrawInfoState {

}

impl<'a> canvas::Program<Message> for HoverList {
    type State = DrawInfoState;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<Message>) {
        println!("{:?} {:?}", bounds, event);

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
            println!("{:?} ", bounds);
            let background = canvas::Path::rectangle(Point::ORIGIN, frame.size());
            frame.fill(&background, iced::Color::from_rgb8(0, 0, 0));

        });

        let mut result = Vec::new();
        result.push(content);
        result
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(&bounds) {
            mouse::Interaction::Text
        } else {
            mouse::Interaction::default()
        }
    }
}
