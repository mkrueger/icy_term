use std::cmp::{max, min};

use egui::Vec2;
use icy_engine::{Buffer, BufferParser, CallbackAction, Caret, EngineResult, Position};

pub mod selection;
pub use selection::*;

use crate::{MonitorSettings, Options, Scaling};

mod output_renderer;
mod sixel_renderer;
mod terminal_renderer;

#[derive(Clone, Copy)]
pub enum BufferInputMode {
    CP437,
    PETscii,
    ATAscii,
    ViewData,
}

pub struct Blink {
    is_on: bool,
    last_blink: u128,
    blink_rate: u128,
}

impl Blink {
    pub fn new(blink_rate: u128) -> Self {
        Self {
            is_on: false,
            last_blink: 0,
            blink_rate,
        }
    }

    pub fn is_on(&self) -> bool {
        self.is_on
    }

    pub fn update(&mut self, cur_ms: u128) -> bool {
        if cur_ms - self.last_blink > self.blink_rate {
            self.is_on = !self.is_on;
            self.last_blink = cur_ms;
            true
        } else {
            false
        }
    }
}

pub struct BufferView {
    pub buf: Buffer,
    pub caret: Caret,

    pub scale: f32,
    pub buffer_input_mode: BufferInputMode,
    pub viewport_top: f32,

    pub char_size: Vec2,

    pub button_pressed: bool,

    selection_opt: Option<Selection>,

    scaling: Scaling,
    pub monitor_settings: MonitorSettings,

    terminal_renderer: terminal_renderer::TerminalRenderer,
    sixel_renderer: sixel_renderer::SixelRenderer,
    output_renderer: output_renderer::OutputRenderer,
}

impl BufferView {
    pub fn new(gl: &glow::Context, options: &Options) -> Self {
        let mut buf = Buffer::create(80, 25);
        buf.layers[0].is_transparent = false;
        buf.is_terminal_buffer = true;

        let terminal_renderer = terminal_renderer::TerminalRenderer::new(gl);
        let filter = match options.scaling {
            Scaling::Nearest => glow::NEAREST as i32,
            Scaling::Linear => glow::LINEAR as i32,
        };
        let sixel_renderer = sixel_renderer::SixelRenderer::new(gl, &buf, filter);
        let output_renderer = output_renderer::OutputRenderer::new(gl, &buf, filter);
        Self {
            buf,
            caret: Caret::default(),
            scale: 1.0,
            buffer_input_mode: BufferInputMode::CP437,
            button_pressed: false,
            viewport_top: 0.,
            selection_opt: None,
            scaling: options.scaling,
            monitor_settings: options.monitor_settings.clone(),
            char_size: Vec2::ZERO,
            terminal_renderer,
            sixel_renderer,
            output_renderer,
        }
    }

    pub fn get_selection(&mut self) -> &mut Option<Selection> {
        &mut self.selection_opt
    }

    pub fn set_selection(&mut self, sel: Selection) {
        self.selection_opt = Some(sel);
    }

    pub fn clear_selection(&mut self) {
        self.selection_opt = None;
    }

    pub fn clear(&mut self) {
        self.caret.ff(&mut self.buf);
    }

    pub fn get_copy_text(&mut self, buffer_parser: &dyn BufferParser) -> Option<String> {
        let Some(selection) = &self.selection_opt else {
            return None;
        };

        let mut res = String::new();
        if selection.block_selection {
            let start = Position::new(
                min(selection.anchor_pos.x, selection.lead_pos.x),
                min(selection.anchor_pos.y, selection.lead_pos.y),
            );
            let end = Position::new(
                max(selection.anchor_pos.x, selection.lead_pos.x),
                max(selection.anchor_pos.y, selection.lead_pos.y),
            );
            for y in start.y..=end.y {
                for x in start.x..end.x {
                    let ch = self.buf.get_char(Position::new(x, y)).unwrap();
                    res.push(buffer_parser.convert_to_unicode(ch.ch));
                }
                res.push('\n');
            }
        } else {
            let (start, end) = if selection.anchor_pos < selection.lead_pos {
                (selection.anchor_pos, selection.lead_pos)
            } else {
                (selection.lead_pos, selection.anchor_pos)
            };
            if start.y == end.y {
                for x in start.x..=end.x {
                    let ch = self.buf.get_char(Position::new(x, start.y)).unwrap();
                    res.push(buffer_parser.convert_to_unicode(ch.ch));
                }
            } else {
                for x in start.x..self.buf.get_line_length(start.y) {
                    let ch = self.buf.get_char(Position::new(x, start.y)).unwrap();
                    res.push(buffer_parser.convert_to_unicode(ch.ch));
                }
                res.push('\n');
                for y in start.y + 1..end.y {
                    for x in 0..self.buf.get_line_length(y) {
                        let ch = self.buf.get_char(Position::new(x, y)).unwrap();
                        res.push(buffer_parser.convert_to_unicode(ch.ch));
                    }
                    res.push('\n');
                }
                for x in 0..=end.x {
                    let ch = self.buf.get_char(Position::new(x, end.y)).unwrap();
                    res.push(buffer_parser.convert_to_unicode(ch.ch));
                }
            }
        }
        self.selection_opt = None;
        Some(res)
    }

    pub fn redraw_view(&mut self) {
        self.terminal_renderer.redraw_terminal();
    }

    pub fn redraw_palette(&mut self) {
        self.terminal_renderer.redraw_palette();
    }

    pub fn redraw_font(&mut self) {
        self.terminal_renderer.redraw_font();
    }

    pub fn print_char(
        &mut self,
        parser: &mut Box<dyn BufferParser>,
        c: char,
    ) -> EngineResult<CallbackAction> {
        let res = parser.print_char(&mut self.buf, &mut self.caret, c);
        self.redraw_view();
        res
    }

    pub fn render_contents(
        &mut self,
        gl: &glow::Context,
        info: &egui::PaintCallbackInfo,
        rect: egui::Rect,
    ) {
        self.update_contents(gl);
        unsafe {
            self.output_renderer.init_output(gl);
            self.terminal_renderer.render_terminal(gl, self);
            // draw sixels
            let render_texture = self
                .sixel_renderer
                .render_sixels(gl, self, &self.output_renderer);
            self.output_renderer.render_to_screen(
                gl,
                info,
                render_texture,
                rect,
                &self.monitor_settings,
            );
        }
    }

    pub fn update_contents(&mut self, gl: &glow::Context) {
        let scale_filter = match self.scaling {
            Scaling::Nearest => glow::NEAREST as i32,
            Scaling::Linear => glow::LINEAR as i32,
        };

        self.sixel_renderer
            .update_sixels(gl, &mut self.buf, scale_filter);
        self.terminal_renderer.update_textures(
            gl,
            &mut self.buf,
            self.viewport_top,
            self.char_size,
        );
        self.output_renderer
            .update_render_buffer(gl, &self.buf, scale_filter);
    }

    pub fn destroy(&self, gl: &glow::Context) {
        self.terminal_renderer.destroy(gl);
        self.output_renderer.destroy(gl);
        self.sixel_renderer.destroy(gl);
    }

    pub fn set_scaling(&mut self, scaling: Scaling) {
        self.scaling = scaling;
    }
}

const SHADER_SOURCE: &str = r#"#version 300 es
precision highp float;

const float low  = -1.0;
const float high = 1.0;

void main() {
    vec2 vert = vec2(0, 0);
    switch (gl_VertexID) {
        case 0:
            vert = vec2(low, high);
            break;
        case 1:
            vert = vec2(high, high);
            break;
        case 2:
            vert = vec2(high, low);
            break;
        case 3:
            vert = vec2(low, high);
            break;
        case 4:
            vert = vec2(low, low);
            break;
        case 5:
            vert = vec2(high, low);
            break;
    }
    gl_Position = vec4(vert, 0.3, 1.0);
}
"#;
