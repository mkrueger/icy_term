use std::cmp::max;
use std::io;
use crate::com::Com;
use crate::model::{Buffer, Position, TextAttribute, DosChar, parse_ansi, parse_petscii};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{
    self, Cursor, Frame, Geometry,
};
use iced::{ Point, Rectangle, Theme};

use super::main_window::Message;

pub static mut SCALE: f32 = 1.0;

pub struct BufferView {
    pub buf: Buffer,
    pub caret: Position,
    pub attr: TextAttribute,
    pub cache: canvas::Cache,
    state: crate::model::ParseStates,
    pstate: crate::model::PETSCIIState,
    pub blink: bool,
    pub last_blink: u128,
    pub scale: f32
}

impl BufferView {
    pub fn new() -> Self {
        Self {
            buf: Buffer::create(80, 25),
            caret: Position::new(),
            attr: TextAttribute::DEFAULT,
            cache: canvas::Cache::default(),
            state: crate::model::ParseStates::new(),
            pstate: crate::model::PETSCIIState::new(),
            blink: false,
            last_blink: 0,
            scale: 1.0
        }
    }

    pub fn print_char<T: Com>(&mut self, telnet: &mut T, c: u8) -> io::Result<()>
    {
        let c = if self.buf.petscii { parse_petscii(&mut self.buf, &mut self.caret, &mut self.attr, &mut self.pstate, telnet, c) } else { parse_ansi(&mut self.buf, &mut self.caret, &mut self.attr, &mut self.state, telnet, c) };

        if let Err(err) = &c {
            println!("error in ansi sequence: {}", err);
        }

        if let Ok(Some(ch)) = c {
            match ch {
                10 => {
                    self.caret.x = 0;
                    self.caret.y += 1;
                }
                12 => {
                    self.caret.x = 0;
                    self.caret.y = 1;
                    self.attr = TextAttribute::DEFAULT;
                }
                13 => {
                    self.caret.x = 0;
                }
                8 => {
                    self.caret.x = max(0, self.caret.x - 1);
                }
                _ => {
                    let mut ch = DosChar::from(ch as u16, self.attr);
                    if self.buf.petscii {
                        ch.ext_font = self.pstate.ext_font;
                    }
                    self.buf.set_char(self.caret, Some(ch));
                    self.caret.x = self.caret.x + 1;
                    if self.caret.x >= self.buf.width as i32 {
                        self.caret.x = 0;
                        self.caret.y = self.caret.y + 1;
                    }
                }
            };

            if self.caret.y >= self.buf.height as i32 {
                self.buf.layer.remove_line(0);
                self.buf.layer.insert_line(self.buf.height as i32 - 1, crate::model::Line::new());
                self.caret.y = self.buf.height as i32 - 1;
                self.buf.clear_line(self.caret.y);
            }
        }
        Ok(())
    }
}


#[derive(Default, Debug, Clone, Copy)]
pub struct DrawInfoState {
}


impl<'a> canvas::Program<Message> for BufferView {
    type State = DrawInfoState;

    fn update(
        &self,
        _state: &mut Self::State,
        _event: Event,
        _bounds: Rectangle,
        _cursor: Cursor,
    ) -> (event::Status, Option<Message>) {

        (event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &Self::State,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {

        let content =
            self.cache.draw(bounds.size(), |frame: &mut Frame| {
                let background = canvas::Path::rectangle(Point::ORIGIN, frame.size());
                frame.fill(&background, iced::Color::from_rgb8(0x40, 0x44, 0x4B));
            
                let buffer = &self.buf;
                let font_dimensions = buffer.get_font_dimensions();
                let mut char_size = iced::Size::new(font_dimensions.width as f32, font_dimensions.height as f32);

                let mut w = buffer.width as f32 * char_size.width;
                let mut h = buffer.height as f32 * char_size.height;

                let double_mode = w * 2.0 <= bounds.width && h * 2.0 <= bounds.height;
                if double_mode {
                    char_size.width *= 2.0;
                    char_size.height *= 2.0;
                    w = buffer.width as f32 * char_size.width;
                    h = buffer.height as f32 * char_size.height;
                    unsafe { SCALE = 2.0; }
                }  else { 
                    unsafe {
                        SCALE = 1.0;
                    }
                }

                let top_x = (bounds.width - w) / 2.0;
                let top_y = (bounds.height - h) / 2.0;
               // println!("{:?} b: {}x{} = {} / {}", bounds, w, h, top_x, top_y);

                for y in 0..buffer.height as usize {
                    for x in 0..buffer.width as usize {
                        let rect  = Rectangle::new(
                            Point::new(
                                top_x + (x * char_size.width as usize) as f32 + 0.5,  
                                top_y + (y * char_size.height as usize) as f32 + 0.5), 
                                char_size
                            );
                            if let Some(ch) = buffer.get_char(crate::model::Position::from(x as i32, y as i32)) {
                                let bg = buffer.palette.colors[ch.attribute.get_background() as usize];
                                let (r, g, b) = bg.get_rgb_f32();

                                let color = iced::Color::new(r, g, b, 1.0);
                                frame.fill_rectangle(rect.position(), rect.size(), color);

                                let fg = buffer.palette.colors[ch.attribute.get_foreground() as usize];
                                let (r, g, b) = fg.get_rgb_f32();
                                let color = iced::Color::new(r, g, b, 1.0);
                                for y in 0..font_dimensions.height {
                                    let line = buffer.get_font_scanline(ch.char_code, y as usize);
                                    for x in 0..font_dimensions.width {
                                        if (line & (128 >> x)) != 0 {
                                            if double_mode {
                                                frame.fill_rectangle(Point::new(rect.x + x as f32 * 2.0, rect.y + y as f32 * 2.0), iced::Size::new(2_f32, 2_f32), color);
                                            } else {
                                                frame.fill_rectangle(Point::new(rect.x + x as f32, rect.y + y as f32), iced::Size::new(1_f32, 1_f32), color);
                                            }
                                        }
                                    }
                                }
                            }
                    }
                }
            });

            if !self.blink {
                let buffer = &self.buf;
                let font_dimensions = buffer.get_font_dimensions();
                let mut char_size = iced::Size::new(font_dimensions.width as f32, font_dimensions.height as f32);

                let mut w = buffer.width as f32 * char_size.width;
                let mut h = buffer.height as f32 * char_size.height;

                let double_mode = w * 2.0 <= bounds.width && h * 2.0 <= bounds.height;
                if double_mode {
                    char_size.width *= 2.0;
                    char_size.height *= 2.0;
                    w = buffer.width as f32 * char_size.width;
                    h = buffer.height as f32 * char_size.height;
                }
                let top_x = (bounds.width - w) / 2.0;
                let top_y = (bounds.height - h) / 2.0;
                

                let caret_size = iced::Size::new(char_size.width, char_size.height / 8.0);
                let p = Point::new(
                    top_x + (self.caret.x as f32 * char_size.width) as f32 + 0.5,  
                    top_y + (self.caret.y as f32 * char_size.height) as f32 + 0.5 + char_size.height - caret_size.height);
                let caret = canvas::Path::rectangle(p, caret_size);
                let mut frame = Frame::new(bounds.size());

                let bg = buffer.palette.colors[self.attr.get_foreground() as usize];
                let (r, g, b) = bg.get_rgb_f32();

                frame.fill(&caret, iced::Color::new(r, g, b, 1.0));

                vec![content, frame.into_geometry()]
            } else {
                vec![content]
            }
    }
}