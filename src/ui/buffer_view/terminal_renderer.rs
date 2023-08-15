#![allow(clippy::float_cmp)]
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use egui::epaint::ahash::HashMap;
use glow::HasContext as _;
use glow::NativeProgram;
use glow::NativeTexture;
use icy_engine::Buffer;

use super::Blink;
use super::ViewState;

pub struct TerminalRenderer {
    terminal_shader: glow::Program,

    font_lookup_table: HashMap<usize, usize>,

    terminal_render_texture: NativeTexture,
    font_texture: NativeTexture,
    palette_texture: NativeTexture,
    vertex_array: glow::VertexArray,

    // used to determine if palette needs to be updated - Note: palette only grows.
    old_palette_color_count: usize,

    redraw_view: bool,
    redraw_palette: bool,
    redraw_font: bool,

    caret_blink: Blink,
    character_blink: Blink,
}

impl TerminalRenderer {
    pub fn new(gl: &glow::Context) -> Self {
        unsafe {
            let font_texture = create_font_texture(gl);
            let palette_texture = create_palette_texture(gl);
            let terminal_render_texture = create_buffer_texture(gl);
            let terminal_shader = compile_shader(gl);
            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            Self {
                terminal_shader,
                font_lookup_table: HashMap::default(),
                old_palette_color_count: 0,

                terminal_render_texture,
                font_texture,
                palette_texture,

                redraw_view: true,
                redraw_palette: true,
                redraw_font: true,
                vertex_array,
                caret_blink: Blink::new((1000.0 / 1.875) as u128 / 2),
                character_blink: Blink::new((1000.0 / 1.8) as u128),
            }
        }
    }

    pub fn redraw_terminal(&mut self) {
        self.redraw_view = true;
    }

    pub fn redraw_palette(&mut self) {
        self.redraw_palette = true;
    }

    pub fn redraw_font(&mut self) {
        self.redraw_font = true;
    }

    pub fn update_textures(&mut self, gl: &glow::Context, buf: &mut Buffer, scroll_back_line: i32) {
        self.check_blink_timers();

        if self.redraw_font || buf.is_font_table_updated() {
            self.redraw_font = false;
            buf.set_font_table_is_updated();
            self.update_font_texture(gl, buf);
        }

        if self.redraw_view {
            self.redraw_view = false;
            self.update_terminal_texture(gl, buf, scroll_back_line);
        }

        if self.redraw_palette || self.old_palette_color_count != buf.palette.colors.len() {
            self.redraw_palette = false;
            self.old_palette_color_count = buf.palette.colors.len();
            self.update_palette_texture(gl, buf);
        }
    }

    // Redraw whole terminal on caret or character blink update.
    fn check_blink_timers(&mut self) {
        let start: SystemTime = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let cur_ms = since_the_epoch.as_millis();
        if self.caret_blink.update(cur_ms) || self.character_blink.update(cur_ms) {
            self.redraw_terminal();
        }
    }

    fn update_font_texture(&mut self, gl: &glow::Context, buf: &Buffer) {
        let size = buf.get_font(0).unwrap().size;
        let w = size.width as usize;
        let h = size.height as usize;

        let mut font_data = Vec::new();
        let chars_in_line = 16;
        let line_width = w * chars_in_line * 4;
        let height = h * 256 / chars_in_line;
        self.font_lookup_table.clear();
        font_data.resize(line_width * height * buf.font_count(), 0);

        for (cur_font_num, font) in buf.font_iter().enumerate() {
            self.font_lookup_table.insert(*font.0, cur_font_num);
            let fontpage_start = cur_font_num * line_width * height;
            for ch in 0..256usize {
                let cur_font = font.1;
                let glyph = cur_font
                    .get_glyph(unsafe { char::from_u32_unchecked(ch as u32) })
                    .unwrap();

                let x = ch % chars_in_line;
                let y = ch / chars_in_line;

                let offset = x * w * 4 + y * h * line_width + fontpage_start;
                let last_scan_line = h.min(cur_font.size.height as usize);
                for y in 0..last_scan_line {
                    if let Some(scan_line) = glyph.data.get(y) {
                        let mut po = offset + y * line_width;

                        for x in 0..w {
                            if scan_line & (128 >> x) == 0 {
                                po += 4;
                            } else {
                                // unroll
                                font_data[po] = 0xFF;
                                po += 1;
                                font_data[po] = 0xFF;
                                po += 1;
                                font_data[po] = 0xFF;
                                po += 1;
                                font_data[po] = 0xFF;
                                po += 1;
                            }
                        }
                    } else {
                        eprintln!("error in font {} can't get line {y}", font.0);
                        font_data.extend(vec![0xFF; w * 4]);
                    }
                }
            }
        }

        unsafe {
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.font_texture));
            gl.tex_image_3d(
                glow::TEXTURE_2D_ARRAY,
                0,
                glow::RGB as i32,
                line_width as i32 / 4,
                height as i32,
                buf.font_count() as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&font_data),
            );
        }
    }

    fn update_palette_texture(&self, gl: &glow::Context, buf: &Buffer) {
        let mut palette_data = Vec::new();
        for i in 0..buf.palette.colors.len() {
            let (r, g, b) = buf.palette.colors[i].get_rgb();
            palette_data.push(r);
            palette_data.push(g);
            palette_data.push(b);
            palette_data.push(0xFF);
        }
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.palette_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                i32::try_from(glow::RGB).unwrap(),
                i32::try_from(buf.palette.colors.len()).unwrap(),
                1,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&palette_data),
            );
        }
    }

    fn update_terminal_texture(&self, gl: &glow::Context, buf: &Buffer, scroll_back_line: i32) {
        let first_line = 0.max(buf.layers[0].lines.len() as i32 - buf.get_buffer_height());
        let mut buffer_data = Vec::with_capacity(
            2 * buf.get_buffer_width() as usize * 4 * buf.get_real_buffer_height() as usize,
        );
        let colors = buf.palette.colors.len() as u32 - 1;
        let mut y = 0;

        while y < buf.get_buffer_height() {
            let mut is_double_height = false;

            for x in 0..buf.get_buffer_width() {
                let ch = buf
                    .get_char_xy(x, first_line - scroll_back_line + y)
                    .unwrap_or_default();

                if ch.attribute.is_double_height() {
                    is_double_height = true;
                }
                if ch.attribute.is_concealed() {
                    buffer_data.push(b' ');
                } else {
                    buffer_data.push(ch.ch as u8);
                }
                if ch.attribute.is_bold() {
                    buffer_data.push(conv_color(ch.attribute.get_foreground() + 8, colors));
                } else {
                    buffer_data.push(conv_color(ch.attribute.get_foreground(), colors));
                }
                buffer_data.push(conv_color(ch.attribute.get_background(), colors));
                if buf.has_fonts() {
                    let font_number: usize =
                        *self.font_lookup_table.get(&ch.get_font_page()).unwrap();
                    buffer_data.push(font_number as u8);
                } else {
                    buffer_data.push(0);
                }
            }

            if is_double_height {
                for x in 0..buf.get_buffer_width() {
                    let ch = buf
                        .get_char_xy(x, first_line - scroll_back_line + y)
                        .unwrap_or_default();

                    if ch.attribute.is_double_height() {
                        buffer_data.push(ch.ch as u8);
                    } else {
                        buffer_data.push(b' ');
                    }

                    if ch.attribute.is_bold() {
                        buffer_data.push(conv_color(ch.attribute.get_foreground() + 8, colors));
                    } else {
                        buffer_data.push(conv_color(ch.attribute.get_foreground(), colors));
                    }

                    buffer_data.push(conv_color(ch.attribute.get_background(), colors));

                    if buf.has_fonts() {
                        let font_number: usize =
                            *self.font_lookup_table.get(&ch.get_font_page()).unwrap();
                        buffer_data.push(font_number as u8);
                    } else {
                        buffer_data.push(0);
                    }
                }
            }

            if is_double_height {
                y += 2;
            } else {
                y += 1;
            }
        }
        y = 0;
        while y < buf.get_buffer_height() {
            let mut is_double_height = false;

            for x in 0..buf.get_buffer_width() {
                let ch = buf
                    .get_char_xy(x, first_line - scroll_back_line + y)
                    .unwrap_or_default();

                let mut attr = if ch.attribute.is_double_underlined() {
                    3
                } else {
                    u8::from(ch.attribute.is_underlined())
                };
                if ch.attribute.is_crossed_out() {
                    attr |= 4;
                }

                if ch.attribute.is_double_height() {
                    is_double_height = true;
                    attr |= 8;
                }

                buffer_data.push(attr);
                buffer_data.push(attr);
                buffer_data.push(attr);
                buffer_data.push(if ch.attribute.is_blinking() { 255 } else { 0 });
            }

            if is_double_height {
                for x in 0..buf.get_buffer_width() {
                    let ch = buf
                        .get_char_xy(x, first_line - scroll_back_line + y)
                        .unwrap_or_default();
                    let mut attr = if ch.attribute.is_double_underlined() {
                        3
                    } else {
                        u8::from(ch.attribute.is_underlined())
                    };
                    if ch.attribute.is_crossed_out() {
                        attr |= 4;
                    }

                    if ch.attribute.is_double_height() {
                        is_double_height = true;
                        attr |= 8;
                        attr |= 16;
                    }

                    buffer_data.push(attr);
                    buffer_data.push(attr);
                    buffer_data.push(attr);
                    buffer_data.push(if ch.attribute.is_blinking() { 255 } else { 0 });
                }
            }

            if is_double_height {
                y += 2;
            } else {
                y += 1;
            }
        }
        unsafe {
            gl.active_texture(glow::TEXTURE0 + 4);
            gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(self.terminal_render_texture));
            gl.tex_image_3d(
                glow::TEXTURE_2D_ARRAY,
                0,
                glow::RGBA32F as i32,
                buf.get_buffer_width(),
                buf.get_buffer_height(),
                2,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(&buffer_data),
            );
        }
    }

    pub(crate) fn render_terminal(&self, gl: &glow::Context, view_state: &ViewState) {
        unsafe {
            self.run_shader(gl, view_state);

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.font_texture));
            gl.active_texture(glow::TEXTURE0 + 2);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.palette_texture));
            gl.active_texture(glow::TEXTURE0 + 4);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.terminal_render_texture));

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.draw_arrays(glow::TRIANGLES, 3, 3);
        }
    }

    unsafe fn run_shader(&self, gl: &glow::Context, view_state: &ViewState) {
        gl.use_program(Some(self.terminal_shader));
        gl.uniform_2_f32(
            gl.get_uniform_location(self.terminal_shader, "u_resolution")
                .as_ref(),
            view_state.render_buffer_size.x,
            view_state.render_buffer_size.y,
        );
        gl.uniform_2_f32(
            gl.get_uniform_location(self.terminal_shader, "u_position")
                .as_ref(),
            0.0,
            0.0,
        );

        let sbl = (view_state.buf.get_first_visible_line() - view_state.scroll_back_line) as f32;
        gl.uniform_4_f32(
            gl.get_uniform_location(self.terminal_shader, "u_caret_position")
                .as_ref(),
            view_state.caret.get_position().x as f32,
            view_state.caret.get_position().y as f32 - sbl,
            if self.caret_blink.is_on() && view_state.caret.is_visible {
                1.0
            } else {
                0.0
            },
            if view_state.caret.insert_mode {
                1.0
            } else {
                0.0
            }, // shape
        );

        gl.uniform_1_f32(
            gl.get_uniform_location(self.terminal_shader, "u_blink")
                .as_ref(),
            if self.character_blink.is_on() {
                1.0
            } else {
                0.0
            },
        );

        gl.uniform_2_f32(
            gl.get_uniform_location(self.terminal_shader, "u_terminal_size")
                .as_ref(),
            view_state.buf.get_buffer_width() as f32 - 0.0001,
            view_state.buf.get_buffer_height() as f32 - 0.0001,
        );

        gl.uniform_1_i32(
            gl.get_uniform_location(self.terminal_shader, "u_fonts")
                .as_ref(),
            0,
        );
        gl.uniform_1_i32(
            gl.get_uniform_location(self.terminal_shader, "u_palette")
                .as_ref(),
            2,
        );
        gl.uniform_1_i32(
            gl.get_uniform_location(self.terminal_shader, "u_buffer")
                .as_ref(),
            4,
        );

        match &view_state.selection_opt {
            Some(selection) => {
                if selection.is_empty() {
                    gl.uniform_4_f32(
                        gl.get_uniform_location(self.terminal_shader, "u_selection")
                            .as_ref(),
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                    gl.uniform_1_f32(
                        gl.get_uniform_location(self.terminal_shader, "u_selection_attr")
                            .as_ref(),
                        -1.0,
                    );
                } else {
                    if selection.anchor.y.floor() < selection.lead.y.floor()
                        || selection.anchor.y.floor() == selection.lead.y.floor()
                            && selection.anchor.x < selection.lead.x
                    {
                        gl.uniform_4_f32(
                            gl.get_uniform_location(self.terminal_shader, "u_selection")
                                .as_ref(),
                            selection.anchor.x.floor(),
                            selection.anchor.y.floor() - sbl,
                            selection.lead.x.floor(),
                            selection.lead.y.floor() - sbl,
                        );
                    } else {
                        gl.uniform_4_f32(
                            gl.get_uniform_location(self.terminal_shader, "u_selection")
                                .as_ref(),
                            selection.lead.x.floor(),
                            selection.lead.y.floor() - sbl,
                            selection.anchor.x.floor(),
                            selection.anchor.y.floor() - sbl,
                        );
                    }
                    if selection.block_selection {
                        gl.uniform_1_f32(
                            gl.get_uniform_location(self.terminal_shader, "u_selection_attr")
                                .as_ref(),
                            1.0,
                        );
                    } else {
                        gl.uniform_1_f32(
                            gl.get_uniform_location(self.terminal_shader, "u_selection_attr")
                                .as_ref(),
                            0.0,
                        );
                    }
                }
            }
            None => {
                gl.uniform_4_f32(
                    gl.get_uniform_location(self.terminal_shader, "u_selection")
                        .as_ref(),
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                );
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.terminal_shader, "u_selection_attr")
                        .as_ref(),
                    -1.0,
                );
            }
        }
    }

    pub(crate) fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_vertex_array(self.vertex_array);

            gl.delete_program(self.terminal_shader);

            gl.delete_texture(self.terminal_render_texture);
            gl.delete_texture(self.font_texture);
            gl.delete_texture(self.palette_texture);
        }
    }
}

unsafe fn compile_shader(gl: &glow::Context) -> NativeProgram {
    let program = gl.create_program().expect("Cannot create program");

    let (vertex_shader_source, fragment_shader_source) = (
        r#"#version 330
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
        include_str!("terminal_renderer.shader.frag"),
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
            gl.shader_source(
                shader,
                shader_source, /*&format!("{}\n{}", shader_version, shader_source)*/
            );
            gl.compile_shader(shader);
            assert!(
                gl.get_shader_compile_status(shader),
                "{}",
                gl.get_shader_info_log(shader)
            );
            gl.attach_shader(program, shader);
            shader
        })
        .collect();

    gl.link_program(program);
    assert!(
        gl.get_program_link_status(program),
        "{}",
        gl.get_program_info_log(program)
    );

    for shader in shaders {
        gl.detach_shader(program, shader);
        gl.delete_shader(shader);
    }
    program
}

unsafe fn create_buffer_texture(gl: &glow::Context) -> NativeTexture {
    let buffer_texture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(buffer_texture));
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_MIN_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_MAG_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
    );
    buffer_texture
}

unsafe fn create_palette_texture(gl: &glow::Context) -> NativeTexture {
    let palette_texture: NativeTexture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D, Some(palette_texture));

    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
    );

    palette_texture
}

unsafe fn create_font_texture(gl: &glow::Context) -> NativeTexture {
    let font_texture = gl.create_texture().unwrap();
    gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(font_texture));

    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_MIN_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_MAG_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D_ARRAY,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
    );
    font_texture
}

fn conv_color(c: u32, colors: u32) -> u8 {
    ((255 * c) / colors) as u8
}
