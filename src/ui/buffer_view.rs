use crate::com::Com;
use crate::sound::play_music;
use clipboard::{ClipboardContext, ClipboardProvider};
use icy_engine::{AvatarParser, Buffer, BufferParser, Caret, Position, SixelReadStatus, CallbackAction, EngineResult};
use std::cmp::{max, min};
use eframe::epaint::{ Rect, Vec2};
use glow::NativeTexture;

#[derive(Clone, Copy)]
pub enum BufferInputMode {
    CP437,
    PETSCII,
    ATASCII,
    VT500,
    VIEWDATA
}

impl BufferInputMode {
    pub fn cur_map<'a>(&self) -> &'a [(u32, &[u8])] {
        match self {
            super::BufferInputMode::CP437 => super::ANSI_KEY_MAP,
            super::BufferInputMode::PETSCII => super::C64_KEY_MAP,
            super::BufferInputMode::ATASCII => super::ATASCII_KEY_MAP,
            super::BufferInputMode::VT500 => super::VT500_KEY_MAP,
            super::BufferInputMode::VIEWDATA => super::VIDEOTERM_KEY_MAP,
        }
    }
}

struct SixelCacheEntry {
    pub status: SixelReadStatus,
    pub old_line: i32,
    pub data_opt: Option<Vec<u8>>,

    pub pos: Position,
    pub size: icy_engine::Size<i32>,
}

impl SixelCacheEntry {
    pub fn rect(&self) -> icy_engine::Rectangle {
        icy_engine::Rectangle {
            start: self.pos,
            size: self.size,
        }
    }
}

pub struct BufferView {
    pub buf: Buffer,
    sixel_cache: Vec<SixelCacheEntry>,
    pub caret: Caret,
    pub blink: bool,
    pub last_blink: u128,
    pub scale: f32,
    pub buffer_input_mode: BufferInputMode,

    pub button_pressed: bool,

    program: glow::Program,
    vertex_array: glow::VertexArray,
    
    redraw_view: bool,
    font_texture: NativeTexture,
    buffer_texture: NativeTexture,
    palette_texture: NativeTexture
}

impl BufferView {
    pub fn new(gl: &glow::Context) -> Self {
        let mut buf = Buffer::create(80, 25);
        buf.layers[0].is_transparent = false;
        buf.is_terminal_buffer = true;

        use glow::HasContext as _;

        let shader_version = if cfg!(target_arch = "wasm32") {
            "#version 300 es"
        } else {
            "#version 330"
        };

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let mut shader = "precision highp float;\n".to_string();

            shader.push_str(r#"
            uniform sampler2DArray u_fonts;
            uniform sampler2D u_palette;
            uniform sampler2D u_buffer;
                        
            uniform vec2        u_resolution;
            uniform vec2        u_position;
            uniform vec2        u_buffer_size;

            uniform vec2        u_viewport_size;
            
            out     vec4        fragColor;
            
            vec4 get_char(vec2 p, float c, float page) {
                if (p.x < 0.|| p.x > 1. || p.y < 0.|| p.y > 1.) return vec4(0, 0, 0, 1.0);
                vec2 v = vec2(
                    p.x / 16 + fract ( floor (c) / 16 ),
                    p.y / 16 + fract ( floor (-15.999 + c / 16) / 16 )
                );
                
                return texture( u_fonts, vec3(v, page) );
                // return textureGrad( u_fonts, vec3(v, page), dFdx(p/16.),dFdy(p/16.) );
            }

            // TODO: Is there an easier way to disable the 'intelligent' color conversion crap?
            vec4 toLinearSlow(vec4 col) {
                bvec4 cutoff = lessThan(col, vec4(0.04045));
                vec4 higher = vec4(pow((col.rgb + vec3(0.055))/vec3(1.055), vec3(2.4)), col.a);
                vec4 lower = vec4(col.rgb/vec3(12.92), col.a);
                return mix(higher, lower, cutoff);
            }
                        
            vec4 get_palette_color(float c) {
                return toLinearSlow(texture(u_palette, vec2(c, 0)));
            }
            
            void main (void) {
                vec2 view_coord = gl_FragCoord.xy / u_viewport_size;
                view_coord = vec2(view_coord.s, 1.0 - view_coord.t);
                view_coord -= u_position / u_viewport_size;

                vec2 sz = u_buffer_size;
                vec2 fb_pos = (view_coord * sz);

                float z  = u_resolution.y / u_viewport_size.y;
                vec4 ch = texture(u_buffer, vec2(view_coord.x, view_coord.y / z));
                
                fb_pos = fract(vec2(fb_pos.x, fb_pos.y / z));
                
                vec4 char_data = get_char(fb_pos, ch.x * 255, 0);

                if (char_data.x > 0.5) {
                    fragColor = get_palette_color(ch.y);
                } else {
                    fragColor = get_palette_color(ch.z);
                }
            }"#);

            let (vertex_shader_source, fragment_shader_source) = (
r#"
const float low  =  -1.0;
const float high = 1.0;

const vec2 verts[6] = vec2[6](
    vec2(low, high),
    vec2(high, high),
    vec2(high, low),

    vec2(low, high),
    vec2(low, low),
    vec2(high, low)
);

void main() {
    vec2 vert = verts[gl_VertexID];
    gl_Position = vec4(vert, 0.3, 1.0);
}
"#,
                shader.as_str(),
            );
            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                    gl.compile_shader(shader);
                    if !gl.get_shader_compile_status(shader) {
                        panic!("{}", gl.get_shader_info_log(shader));
                    }
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            
            let mut buffer_data = Vec::with_capacity(buf.get_buffer_width() as usize * 4 * buf.get_real_buffer_height() as usize);
            let colors = buf.palette.colors.len() as u32 - 1;

            let h = max(buf.get_real_buffer_height(), buf.get_buffer_height());
            for y in 0..h {
                for x in 0..buf.get_buffer_width() {
                    let ch = buf.get_char_xy(x, y).unwrap_or_default();
                    buffer_data.push(ch.ch as u8);
                    if ch.attribute.is_bold() {
                        buffer_data.push(conv_color(ch.attribute.get_foreground() + 8, colors));
                    } else {
                        buffer_data.push(conv_color(ch.attribute.get_foreground(), colors));
                    }
                    buffer_data.push(conv_color(ch.attribute.get_background(), colors));
                    buffer_data.push(ch.get_font_page() as u8);
                }
            }

            let buffer_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(buffer_texture));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGB as i32, buf.get_buffer_width(), buf.get_real_buffer_height(),  0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&buffer_data));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

            let mut palette_data = Vec::new();
            for i in 0..buf.palette.colors.len() {
                let (r, g, b) = buf.palette.colors[i].get_rgb();
                palette_data.push(r);
                palette_data.push(g);
                palette_data.push(b);
                palette_data.push(0xFF);
            }
            let palette_texture = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(palette_texture));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGB as i32, buf.palette.colors.len() as i32, 1, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&palette_data));
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);

            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);

            let w = buf.font_table[0].size.width as usize;
            let h = buf.font_table[0].size.height as usize;
            
            let mut font_data = Vec::new();
            let chars_in_line = 16;
            let line_width = w * chars_in_line * 4;
            let height = h * 256 / chars_in_line;

            font_data.resize(line_width * height, 0);

            for ch in 0..256usize {
                let glyph = buf.font_table[0].get_glyph(char::from_u32_unchecked(ch as u32)).unwrap();

                let x = ch % chars_in_line;
                let y = ch / chars_in_line;

                let offset = x * w * 4 + y * h * line_width;
                for y in 0..h {
                    let scan_line = glyph.data[y];
                    let mut po  = offset + y * line_width;

                    for x in 0..w {
                        if scan_line & (128 >> x) != 0 {
                            // unroll
                            font_data[po] = 0xFF;
                            po += 1;
                            font_data[po] = 0xFF;
                            po += 1;
                            font_data[po] = 0xFF;
                            po += 1;
                            font_data[po] = 0xFF;
                            po += 1;
                        } else {
                            po += 4;
                        }
                    }
                }
            }
            
            let font_texture = gl.create_texture().unwrap();
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(font_texture));
            gl.tex_image_3d(glow::TEXTURE_2D_ARRAY, 0, glow::RGB as i32, line_width as i32 / 4, height as i32, 1, 0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&font_data));
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
            
        Self {
            buf,
            caret: Caret::default(),
            blink: false,
            last_blink: 0,
            scale: 1.0,
            buffer_input_mode: BufferInputMode::CP437,
            sixel_cache: Vec::new(),
            button_pressed: false,
            redraw_view: false,
            program,
            vertex_array,
            font_texture,
            buffer_texture,
            palette_texture
        }
    }
}

    pub fn clear(&mut self) {
        self.caret.ff(&mut self.buf);
    }

   
/* 
    pub fn update_sixels(&mut self) -> bool {
        let buffer = &self.buf;
        let l = buffer.layers[0].sixels.len();
        if l == 0 {
            self.sixel_cache.clear();
        }

        let mut res = false;
        let mut i = 0;
        while i < l {
            let sixel = &buffer.layers[0].sixels[i];

            if sixel.width() == 0 || sixel.height() == 0 {
                i += 1;
                continue;
            }

            let mut old_line = 0;
            let current_line = match sixel.read_status {
                SixelReadStatus::Position(_, y) => y * 6,
                SixelReadStatus::Error | SixelReadStatus::Finished => sixel.height() as i32,
                _ => 0,
            };

            if let Some(entry) = self.sixel_cache.get(i) {
                old_line = entry.old_line;
                if let SixelReadStatus::Position(_, _) = sixel.read_status {
                    if old_line + 5 * 6 >= current_line {
                        i += 1;
                        continue;
                    }
                }
                if entry.status == sixel.read_status {
                    i += 1;
                    continue;
                }
            }
            res = true;
            let data_len = (sixel.height() * sixel.width() * 4) as usize;
            let mut removed_index = -1;
            let mut v = if self.sixel_cache.len() > i {
                let mut entry = self.sixel_cache.remove(i);
                // old_handle = entry.image_opt;
                removed_index = i as i32;
                if let Some(ptr) = &mut entry.data_opt {
                    if ptr.len() < data_len {
                        ptr.resize(data_len, 0);
                    }
                    entry.data_opt.take().unwrap()
                } else {
                    let mut data = Vec::with_capacity(data_len);
                    data.resize(data_len, 0);
                    data
                }
            } else {
                let mut data = Vec::with_capacity(data_len);
                data.resize(data_len, 0);
                data
            };

            let mut i = old_line as usize * sixel.width() as usize * 4;

            for y in old_line..current_line {
                for x in 0..sixel.width() {
                    let column = &sixel.picture[x as usize];
                    let data = if let Some(col) = column.get(y as usize) {
                        if let Some(col) = col {
                            let (r, g, b) = col.get_rgb();
                            [r, g, b, 0xFF]
                        } else {
                            // todo: bg color may differ here
                            [0, 0, 0, 0xFF]
                        }
                    } else {
                        [0, 0, 0, 0xFF]
                    };
                    if i >= v.len() {
                        v.extend_from_slice(&data);
                    } else {
                        v[i] = data[0];
                        v[i + 1] = data[1];
                        v[i + 2] = data[2];
                        v[i + 3] = data[3];
                    }
                    i += 4;
                }
            }
            let (handle_opt, data_opt, clear) = match sixel.read_status {
                SixelReadStatus::Finished | SixelReadStatus::Error => (
                    None,
                    None,
                    true,
                ),
                _ => (None, Some(v), false),
            };

            let new_entry = SixelCacheEntry {
                status: sixel.read_status,
                old_line: current_line,
                data_opt,
                pos: sixel.position,
                size: icy_engine::Size {
                    width: sixel.width() as i32,
                    height: sixel.height() as i32,
                },
            };

            if removed_index < 0 {
                self.sixel_cache.push(new_entry);
                if clear {
                    self.clear_invisible_sixel_cache(self.sixel_cache.len() - 1);
                    break;
                }
            } else {
                self.sixel_cache.insert(removed_index as usize, new_entry);
                if clear {
                    self.clear_invisible_sixel_cache(removed_index as usize);
                    break;
                }
            }

        }
        res
    }
*/
    pub fn clear_invisible_sixel_cache(&mut self, j: usize) {
        // remove cache entries that are removed by the engine
        if j > 0 {
            let cur_rect = self.sixel_cache[j].rect();
            let mut i = j - 1;
            loop {
                let other_rect = self.sixel_cache[i].rect();
                if cur_rect.contains(other_rect) {
                    self.sixel_cache.remove(i);
                    self.buf.layers[0].sixels.remove(i);
                }
                if i == 0 {
                    break;
                }
                i -= 1;
            }
        }
    }

    pub fn copy_to_clipboard(&mut self) {
     /*  let Some(selection) = &self.selection else {
            return;
        };

        let mut res = String::new();
        if selection.block_selection {
            for y in selection.selection_start.y..=selection.selection_end.y {
                for x in selection.selection_start.x..selection.selection_end.x {
                    let ch = self.buf.get_char(Position::new(x, y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.ch));
                }
                res.push('\n');
            }
        } else {
            let (start, end) = if selection.anchor < selection.lead {
                (selection.anchor, selection.lead)
            } else {
                (selection.lead, selection.anchor)
            };
            if start.y != end.y {
                for x in start.x..self.buf.get_line_length(start.y) {
                    let ch = self.buf.get_char(Position::new(x, start.y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.ch));
                }
                res.push('\n');
                for y in start.y + 1..end.y {
                    for x in 0..self.buf.get_line_length(y) {
                        let ch = self.buf.get_char(Position::new(x, y)).unwrap();
                        res.push(self.buffer_parser.to_unicode(ch.ch));
                    }
                    res.push('\n');
                }
                for x in 0..end.x {
                    let ch = self.buf.get_char(Position::new(x, end.y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.ch));
                }
            } else {
                for x in start.x..end.x {
                    let ch = self.buf.get_char(Position::new(x, start.y)).unwrap();
                    res.push(self.buffer_parser.to_unicode(ch.ch));
                }
            }
        }
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Err(err) = ctx.set_contents(res) {
            eprintln!("{}", err);
        }
        self.selection = None; */ 
    }

    pub fn redraw_view(&mut self) {
        self.redraw_view = true;
    }

    pub fn print_char(&mut self, parser: &mut Box<dyn BufferParser>, c: char) -> EngineResult<CallbackAction> {
        self.redraw_view();
        parser.print_char(&mut self.buf, &mut self.caret, c)
    }

    pub fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }


    pub fn update_buffer(&mut self, gl: &glow::Context)
    {
        if !self.redraw_view  { return; }
        self.redraw_view = false;
        let buf = &self.buf;
        let mut buffer_data = Vec::with_capacity(buf.get_buffer_width() as usize * 4 * buf.get_real_buffer_height() as usize);
        let colors = buf.palette.colors.len() as u32 - 1;
        let h = max(buf.get_real_buffer_height(), buf.get_buffer_height());
        for y in 0..h {
            for x in 0..buf.get_buffer_width() {
                let ch = buf.get_char_xy(x, y).unwrap_or_default();
                buffer_data.push(ch.ch as u8);
                if ch.attribute.is_bold() {
                    buffer_data.push(conv_color(ch.attribute.get_foreground() + 8, colors));
                } else {
                    buffer_data.push(conv_color(ch.attribute.get_foreground(), colors));
                }
                buffer_data.push(conv_color(ch.attribute.get_background(), colors));
                buffer_data.push(ch.get_font_page() as u8);
            }
        }
        use glow::HasContext as _;
        unsafe {
            gl.active_texture(glow::TEXTURE0 + 4);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.buffer_texture));
            gl.tex_image_2d(glow::TEXTURE_2D, 0, glow::RGB as i32, buf.get_buffer_width(), buf.get_real_buffer_height(),  0, glow::RGBA, glow::UNSIGNED_BYTE, Some(&buffer_data));
        }
    }

    pub fn paint(&self, gl: &glow::Context, rect: Rect, available_size: Vec2) {
        use glow::HasContext as _;
        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_resolution").as_ref(),
                rect.width(),
                rect.height()
            );

            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_viewport_size").as_ref(),
                available_size.x,
                available_size.y
            );

            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_position").as_ref(),
                rect.left(),
                rect.top() - 20.
            );
            let h = max(self.buf.get_real_buffer_height(), self.buf.get_buffer_height());

            gl.uniform_2_f32 (
                gl.get_uniform_location(self.program, "u_buffer_size").as_ref(),
                self.buf.get_buffer_width() as f32,
                h as f32
            );

            gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_fonts").as_ref(), 0);
            gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_palette").as_ref(), 2);
            gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_buffer").as_ref(), 4);

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.font_texture));
            gl.active_texture(glow::TEXTURE0 + 2);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.palette_texture));
            gl.active_texture(glow::TEXTURE0 + 4);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.buffer_texture));
   
           gl.bind_vertex_array(Some(self.vertex_array));
           gl.draw_arrays(glow::TRIANGLES, 0, 3);
           gl.draw_arrays(glow::TRIANGLES, 3, 3);
        }
    }
}

fn conv_color(c: u32, colors: u32) -> u8 {
    let r = ((255 * c) / colors) as u8;
    r
}