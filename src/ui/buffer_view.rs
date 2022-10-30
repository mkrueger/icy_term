use std::io;
use crate::com::Com;
use crate::model::{Buffer, BufferParser, Caret};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{
    self, Cursor, Frame, Geometry,
};
use iced::{ Point, Rectangle, Theme};

use super::main_window::Message;

pub static mut SCALE: f32 = 1.0;

pub struct BufferView {
    pub buf: Buffer,
    pub cache: canvas::Cache,
    pub buffer_parser: Box<dyn BufferParser>,
    pub caret: Caret,
    pub blink: bool,
    pub last_blink: u128,
    pub scale: f32,
    pub petscii: bool,
    pub terminal_size: crate::model::Size<u16>
}

impl BufferView {
    pub fn new() -> Self {
        Self {
            buf: Buffer::create(80, 25),
            caret: Caret::new(),
            cache: canvas::Cache::default(),
            buffer_parser: Box::new(crate::model::AnsiParser::new()),
            blink: false,
            last_blink: 0,
            scale: 1.0,
            petscii: false,
            terminal_size: crate::model::Size::from(80, 25)
        }
    }

    pub fn print_char<T: Com>(&mut self, telnet: Option<&mut T>, c: u8) -> io::Result<()>
    {
        let result = self.buffer_parser.print_char(&mut self.buf, &mut self.caret, c)?;
        if let Some(result) = result {
            if let Some(telnet) = telnet {
                println!("Send '{}'", result);
                telnet.write(result.as_bytes())?;
            }
        }
        self.cache.clear();
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

                let mut w = self.terminal_size.width as f32 * char_size.width;
                let mut h = self.terminal_size.height as f32 * char_size.height;

                let double_mode = w * 2.0 <= bounds.width && h * 2.0 <= bounds.height;
                if double_mode {
                    char_size.width *= 2.0;
                    char_size.height *= 2.0;
                    w = self.terminal_size.width as f32 * char_size.width;
                    h = self.terminal_size.height as f32 * char_size.height;
                    unsafe { SCALE = 2.0; }
                }  else { 
                    unsafe {
                        SCALE = 1.0;
                    }
                }

                let top_x = (bounds.width - w) / 2.0;
                let top_y = (bounds.height - h) / 2.0;
               // println!("{:?} b: {}x{} = {} / {}", bounds, w, h, top_x, top_y);
                let first_line = (self.buf.height - self.terminal_size.height) as i32;

                for y in 0..self.terminal_size.height as usize {
                    for x in 0..self.terminal_size.width as usize {
                        let rect  = Rectangle::new(
                            Point::new(
                                top_x + (x * char_size.width as usize) as f32 + 0.5,  
                                top_y + (y * char_size.height as usize) as f32 + 0.5), 
                                char_size
                            );
                            if let Some(ch) = buffer.get_char(crate::model::Position::from(x as i32, first_line + y as i32)) {
                                let (fg, bg) = 
                                    (buffer.palette.colors[ch.attribute.get_foreground() as usize],
                                    buffer.palette.colors[ch.attribute.get_background() as usize])
                                ;
                                
                                let (r, g, b) = bg.get_rgb_f32();

                                let color = iced::Color::new(r, g, b, 1.0);
                                frame.fill_rectangle(rect.position(), rect.size(), color);

                                let (r, g, b) = fg.get_rgb_f32();
                                let color = iced::Color::new(r, g, b, 1.0);
                                if self.buf.extended_font.is_some() {
                                println!("{} {:?}", ch.ext_font, self.buf.extended_font.as_ref().unwrap().name);
                                }
                                for y in 0..font_dimensions.height {
                                    let line = buffer.get_font_scanline(ch.ext_font, ch.char_code, y as usize);
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

                let mut w = self.terminal_size.width as f32 * char_size.width;
                let mut h = self.terminal_size.height as f32 * char_size.height;

                let double_mode = w * 2.0 <= bounds.width && h * 2.0 <= bounds.height;
                if double_mode {
                    char_size.width *= 2.0;
                    char_size.height *= 2.0;
                    w = self.terminal_size.width as f32 * char_size.width;
                    h = self.terminal_size.height as f32 * char_size.height;
                }
                let top_x = (bounds.width - w) / 2.0;
                let top_y = (bounds.height - h) / 2.0;
                

                let caret_size = iced::Size::new(char_size.width, char_size.height / 8.0);
                let p = Point::new(
                    top_x + (self.caret.get_position().x as f32 * char_size.width) as f32 + 0.5,  
                    top_y + (self.caret.get_position().y as f32 * char_size.height) as f32 + 0.5 + char_size.height - caret_size.height);
                let caret = canvas::Path::rectangle(p, caret_size);
                let mut frame = Frame::new(bounds.size());

                let bg = buffer.palette.colors[self.caret.get_attribute().get_foreground() as usize];
                let (r, g, b) = bg.get_rgb_f32();

                frame.fill(&caret, iced::Color::new(r, g, b, 1.0));

                vec![content, frame.into_geometry()]
            } else {
                vec![content]
            }
    }
}