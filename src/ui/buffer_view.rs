use std::cmp::{max, min};
use std::io;
use crate::com::Com;
use clipboard::{ClipboardProvider, ClipboardContext};
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::{
    self, Cursor, Frame, Geometry,
};
use iced::{ Point, Rectangle, Theme, mouse};
use icy_engine::{Buffer, BufferParser, Caret, Position, AvatarParser};

use super::Message;
use super::selection::Selection;

pub enum BufferInputMode {
    CP437,
    PETSCII,
    ATASCII
}


pub struct BufferView {
    pub buf: Buffer,
    pub cache: canvas::Cache,
    pub buffer_parser: Box<dyn BufferParser>,
    pub caret: Caret,
    pub blink: bool,
    pub last_blink: u128,
    pub scale: f32,
    pub petscii: BufferInputMode,
    pub scroll_back_line: i32,

    pub selection: Option<Selection>,
    pub button_pressed: bool,
    pub block_selection: bool
}

impl BufferView {
    pub fn new() -> Self {
        let mut buf = Buffer::create(80, 25);
        buf.layers[0].is_transparent = false;
        buf.is_terminal_buffer = true;
        Self {
            buf,
            caret: Caret::new(),
            cache: canvas::Cache::default(),
            buffer_parser: Box::new(AvatarParser::new(true)),
            blink: false,
            last_blink: 0,
            scale: 1.0,
            petscii: BufferInputMode::CP437,
            scroll_back_line: 0,
            selection: None,
            button_pressed: false,
            block_selection: false
        }
    }

    pub fn scroll(&mut self, lines: i32) {
        
        self.scroll_back_line = max(0, min(self.buf.layers[0].lines.len() as i32 - self.buf.get_buffer_height(), self.scroll_back_line + lines));
    }

    pub fn clear(&mut self)
    {
        self.caret.ff(&mut self.buf);
    }

    pub fn print_char(&mut self, com: Option<&mut dyn Com>, c: u8) -> io::Result<()>
    {
        self.scroll_back_line = 0;
  
        /*match c  {
            b'\\' => print!("\\\\"),
            b'\n' => print!("\\n"),
            b'\r' => print!("\\r"),
            b'\"' => print!("\\\""),
            _ => {
                if c < b' ' || c > b'\x7F' {
                    print!("\\x{:02X}", c as u8);
                } else {
                    print!("{}", char::from_u32(c as u32).unwrap());
                }
            }
        }*/

        let result_opt = self.buffer_parser.print_char(&mut self.buf, &mut self.caret, c)?;
        if let Some(result) = result_opt {
            if let Some(com) = com {
                com.write(result.as_bytes())?;
            }
        }
        self.cache.clear();
        Ok(())
    }

    pub fn copy_to_clipboard(&mut self) {
        let Some(selection) = &self.selection else {
            return;
        };
        
        let mut res = String::new();
        if selection.block_selection {
            for y in selection.selection_start.y..=selection.selection_end.y  {
                for x in selection.selection_start.x..selection.selection_end.x {
                    let ch = self.buf.get_char(Position::from(x, y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.char_code));
                }
                res.push('\n');
            }
        } else {
            let (start, end) = 
            if selection.selection_anchor_start < selection.selection_anchor_end {
                (selection.selection_anchor_start, selection.selection_anchor_end)
            } else {
                (selection.selection_anchor_end, selection.selection_anchor_start)
            };
            if start.y != end.y {
                for x in start.x..self.buf.get_line_length(start.y)  {
                    let ch = self.buf.get_char(Position::from(x, start.y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.char_code));
                }
                res.push('\n');
                for y in start.y + 1..end.y  {
                    for x in 0..self.buf.get_line_length(y) {
                        let ch = self.buf.get_char(Position::from(x, y)).unwrap();
                        res.push(self.buffer_parser.to_unicode(ch.char_code));
                    }
                    res.push('\n');
                }
                for x in 0..end.x  {
                    let ch = self.buf.get_char(Position::from(x, end.y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.char_code));
                }
            } else {
                for x in start.x..end.x  {
                    let ch = self.buf.get_char(Position::from(x, start.y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.char_code));
                }
            }
        }
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Err(err) = ctx.set_contents(res) {
            eprintln!("{}", err);
        }
        self.selection = None;
        self.cache.clear();
    }
}


#[derive(Default, Debug, Clone)]
pub struct DrawInfoState {
    pub selection: Option<Selection>,
    pub button_pressed: bool
}

impl<'a> canvas::Program<Message> for BufferView {
    type State = DrawInfoState;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (event::Status, Option<Message>) {
        let Some(cursor_position) = cursor.position_in(&bounds) else {
            return (event::Status::Ignored, None);
        };

        if let Some(selection) = &mut state.selection {
            let (_, _, _, _, char_size) = calc(&self.buf, &bounds);

            let top_line = (self.buf.get_first_visible_line() - self.scroll_back_line) as f32 * char_size.height.floor();
            selection.update(&self.buf, &bounds, Point {x: cursor_position.x, y: cursor_position.y + top_line});
            selection.block_selection = self.block_selection;
        }

        match event {
            Event::Mouse(mouse_event) => {
                let message = match mouse_event {
                    mouse::Event::ButtonPressed(button) => {
                        match button {
                            mouse::Button::Left => {
                                let (_, _, _, _, char_size) = calc(&self.buf, &bounds);

                                let top_line = (self.buf.get_first_visible_line() - self.scroll_back_line) as f32 * char_size.height.floor();
                                let mut s = Selection::new(Point {x: cursor_position.x, y: cursor_position.y + top_line});
                                s.block_selection = self.block_selection;
                                state.selection = Some(s);
                                state.button_pressed = true;
                                return (event::Status::Captured, None);
                            },
                            mouse::Button::Right => Some(Message::Copy),
                            mouse::Button::Middle => { println!("paste!"); Some(Message::Paste)},
                            _ => None,
                        }
                    }
                    mouse::Event::ButtonReleased(button) => {
                        match button {
                            mouse::Button::Left => {
                                let r = Some(Message::SetSelection(state.selection.clone()));
                                state.button_pressed = false;
                                state.selection = None;
                                r
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                };

                if message.is_some() {
                    return (event::Status::Captured, message);
                }
                return (event::Status::Ignored, None);
            }
            _ => (event::Status::Ignored, None),
        }
    }



    fn draw(
        &self,
        state: &Self::State,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let first_line = max(0, self.buf.layers[0].lines.len() as i32 - self.buf.get_buffer_height());

        let content =
            self.cache.draw(bounds.size(), |frame: &mut Frame| {
                let background = canvas::Path::rectangle(Point::ORIGIN, frame.size());
                frame.fill(&background, iced::Color::from_rgb8(0x40, 0x44, 0x4B));
            
                let buffer = &self.buf;
                let font_dimensions = buffer.get_font_dimensions();
                let (top_x, top_y, scale_x, scale_y, char_size) = calc(buffer, &bounds);
                
                for y in 0..self.buf.get_buffer_height() as usize {
                    for x in 0..self.buf.get_buffer_width() as usize {
                        let rect  = Rectangle::new(
                            Point::new(
                                top_x + (x * char_size.width as usize) as f32 + 0.5,  
                                top_y + (y * char_size.height as usize) as f32 + 0.5), 
                                char_size
                            );
                        if let Some(ch) = buffer.get_char(Position::from(x as i32, first_line - self.scroll_back_line + y as i32)) {
                            let (fg, bg) = 
                                (buffer.palette.colors[ch.attribute.get_foreground() as usize],
                                buffer.palette.colors[ch.attribute.get_background() as usize])
                            ;
                            let (r, g, b) = bg.get_rgb_f32();

                            let color = iced::Color::new(r, g, b, 1.0);
                            frame.fill_rectangle(rect.position(), rect.size(), color);

                            let (r, g, b) = fg.get_rgb_f32();
                            let color = iced::Color::new(r, g, b, 1.0);
                            for y in 0..font_dimensions.height {
                                let line = buffer.get_font_scanline(ch.ext_font, ch.char_code as u8, y as usize);
                                for x in 0..font_dimensions.width {
                                    if (line & (128 >> x)) != 0 {
                                        frame.fill_rectangle(Point::new(rect.x + x as f32 * scale_x, rect.y + y as f32 * scale_y), iced::Size::new(scale_x, scale_y), color);
                                    }
                                }
                            }
                            if ch.attribute.get_is_underlined() {
                                frame.fill_rectangle(Point::new(rect.x, rect.y + rect.height - 1.0), iced::Size::new(rect.width, 1.0), color);
                            }
                        }
                    }
                }
        });

        let top_line = first_line - self.scroll_back_line;
        
        let mut result = Vec::new();
        result.push(content);

        if self.caret.is_visible && !self.blink && (top_line..(top_line + self.buf.get_buffer_height())).contains(&self.caret.get_position().y) {
            let buffer = &self.buf;
            let (top_x, top_y, _, _, char_size) = calc(buffer, &bounds);
            let caret_size = iced::Size::new(char_size.width, char_size.height / 8.0);
            let p = Point::new(
                top_x + (self.caret.get_position().x * char_size.width as i32) as f32 + 0.5,  
                top_y + ((self.caret.get_position().y - top_line) * char_size.height as i32) as f32 + 0.5 + char_size.height - caret_size.height);
            let caret = canvas::Path::rectangle(p, caret_size);
            let mut frame = Frame::new(bounds.size());

            let bg = buffer.palette.colors[self.caret.get_attribute().get_foreground() as usize];
            let (r, g, b) = bg.get_rgb_f32();

            frame.fill(&caret, iced::Color::new(r, g, b, 1.0));
            result.push(frame.into_geometry());
        }
 
        if let Some(selection) = &state.selection {
            result.push(selection.draw(&self.buf, self.scroll_back_line, &bounds));
        } else if let Some(selection) = &self.selection {
            result.push(selection.draw(&self.buf, self.scroll_back_line, &bounds));
        }

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

pub fn calc(buffer: &Buffer, bounds: &Rectangle) -> (f32, f32, f32, f32, iced::Size) {
    let font_dimensions = buffer.get_font_dimensions();

    let mut scale_x = bounds.width / font_dimensions.width as f32 / buffer.get_buffer_width() as f32;
    let mut scale_y = bounds.height / font_dimensions.height as f32 / buffer.get_buffer_height() as f32;

    if scale_x < scale_y {
        scale_y = scale_x;
    } else {
        scale_x = scale_y;
    }

    let char_size = iced::Size::new(font_dimensions.width as f32 * scale_x, font_dimensions.height as f32 * scale_y);
    let w = buffer.get_buffer_width() as f32 * char_size.width;
    let h = buffer.get_buffer_height() as f32 * char_size.height;

    let top_x = (bounds.width - w) / 2.0;
    let top_y = (bounds.height - h) / 2.0;

    (top_x, top_y, scale_x, scale_y, char_size)
}