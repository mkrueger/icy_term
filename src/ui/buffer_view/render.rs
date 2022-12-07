use eframe::epaint::{Rect, Vec2};
use glow::NativeTexture;
use icy_engine::Buffer;
use std::{
    cmp::max,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::ui::main_window::Scaling;

use super::BufferView;

impl BufferView {
    pub fn paint(&self, gl: &glow::Context, rect: Rect) {
        use glow::HasContext as _;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl.framebuffer_texture(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                Some(self.render_texture),
                0,
            );
            gl.bind_texture(glow::TEXTURE_2D, Some(self.render_texture));
            gl.viewport(
                0,
                0,
                self.render_buffer_size.x as i32,
                self.render_buffer_size.y as i32,
            );
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.clear_color(0., 0., 0., 1.0);

            gl.use_program(Some(self.program));
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_resolution")
                    .as_ref(),
                self.render_buffer_size.x,
                self.render_buffer_size.y,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_position").as_ref(),
                0.0,
                0.0,
            );

            let sbl = (self.buf.get_first_visible_line() - self.scroll_back_line) as f32;
            gl.uniform_4_f32(
                gl.get_uniform_location(self.program, "u_caret_position")
                    .as_ref(),
                self.caret.get_position().x as f32,
                self.caret.get_position().y as f32 - sbl,
                if self.caret_blink.is_on() && self.caret.is_visible {
                    1.0
                } else {
                    0.0
                },
                if self.caret.insert_mode { 1.0 } else { 0.0 }, // shape
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_blink").as_ref(),
                if self.character_blink.is_on() {
                    1.0
                } else {
                    0.0
                },
            );

            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "u_terminal_size")
                    .as_ref(),
                self.buf.get_buffer_width() as f32 - 0.0001,
                self.buf.get_buffer_height() as f32 - 0.0001,
            );

            gl.uniform_1_i32(gl.get_uniform_location(self.program, "u_fonts").as_ref(), 0);
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_palette").as_ref(),
                2,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.program, "u_buffer").as_ref(),
                4,
            );

            match &self.selection_opt {
                Some(sel) => {
                    if sel.is_empty() {
                        gl.uniform_4_f32(
                            gl.get_uniform_location(self.program, "u_selection")
                                .as_ref(),
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                        );
                        gl.uniform_1_f32(
                            gl.get_uniform_location(self.program, "u_selection_attr")
                                .as_ref(),
                            -1.0,
                        );
                    } else {
                        if sel.anchor.y.floor() < sel.lead.y.floor()
                            || sel.anchor.y.floor() == sel.lead.y.floor()
                                && sel.anchor.x < sel.lead.x
                        {
                            gl.uniform_4_f32(
                                gl.get_uniform_location(self.program, "u_selection")
                                    .as_ref(),
                                sel.anchor.x.floor(),
                                sel.anchor.y.floor() - sbl,
                                sel.lead.x.floor(),
                                sel.lead.y.floor() - sbl,
                            );
                        } else {
                            gl.uniform_4_f32(
                                gl.get_uniform_location(self.program, "u_selection")
                                    .as_ref(),
                                sel.lead.x.floor(),
                                sel.lead.y.floor() - sbl,
                                sel.anchor.x.floor(),
                                sel.anchor.y.floor() - sbl,
                            );
                        }
                        if sel.block_selection {
                            gl.uniform_1_f32(
                                gl.get_uniform_location(self.program, "u_selection_attr")
                                    .as_ref(),
                                1.0,
                            );
                        } else {
                            gl.uniform_1_f32(
                                gl.get_uniform_location(self.program, "u_selection_attr")
                                    .as_ref(),
                                0.0,
                            );
                        }
                    }
                }
                None => {
                    gl.uniform_4_f32(
                        gl.get_uniform_location(self.program, "u_selection")
                            .as_ref(),
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    );
                    gl.uniform_1_f32(
                        gl.get_uniform_location(self.program, "u_selection_attr")
                            .as_ref(),
                        -1.0,
                    );
                }
            }

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.font_texture));
            gl.active_texture(glow::TEXTURE0 + 2);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.palette_texture));
            gl.active_texture(glow::TEXTURE0 + 4);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.buffer_texture));

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.draw_arrays(glow::TRIANGLES, 3, 3);

            // draw sixels
            let mut render_texture = self.render_texture;
            let mut sixel_render_texture = self.sixel_render_texture;

            for sixel in &self.sixel_cache {
                if let Some(sixel_tex) = sixel.texture_opt {
                    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
                    gl.framebuffer_texture(
                        glow::FRAMEBUFFER,
                        glow::COLOR_ATTACHMENT0,
                        Some(sixel_render_texture),
                        0,
                    );
                    gl.bind_texture(glow::TEXTURE_2D, Some(sixel_render_texture));

                    gl.viewport(
                        0,
                        0,
                        self.render_buffer_size.x as i32,
                        self.render_buffer_size.y as i32,
                    );

                    gl.use_program(Some(self.sixel_shader));
                    gl.uniform_1_i32(
                        gl.get_uniform_location(self.sixel_shader, "u_render_texture")
                            .as_ref(),
                        4,
                    );
                    gl.uniform_1_i32(
                        gl.get_uniform_location(self.sixel_shader, "u_sixel")
                            .as_ref(),
                        2,
                    );

                    gl.active_texture(glow::TEXTURE0 + 4);
                    gl.bind_texture(glow::TEXTURE_2D, Some(render_texture));

                    gl.active_texture(glow::TEXTURE0 + 2);
                    gl.bind_texture(glow::TEXTURE_2D, Some(sixel_tex));

                    gl.uniform_2_f32(
                        gl.get_uniform_location(self.sixel_shader, "u_resolution")
                            .as_ref(),
                        self.render_buffer_size.x,
                        self.render_buffer_size.y,
                    );

                    let x = sixel.pos.x as f32 * self.buf.get_font_dimensions().width as f32;
                    let y = sixel.pos.y as f32 * self.buf.get_font_dimensions().height as f32;

                    let w = sixel.size.width as f32;
                    let h = sixel.size.height as f32;

                    gl.uniform_4_f32(
                        gl.get_uniform_location(self.sixel_shader, "u_sixel_rectangle")
                            .as_ref(),
                        x,
                        y,
                        x + w,
                        y + h,
                    );

                    gl.bind_vertex_array(Some(self.vertex_array));
                    gl.draw_arrays(glow::TRIANGLES, 0, 3);
                    gl.draw_arrays(glow::TRIANGLES, 3, 3);

                    let tmp = render_texture;
                    render_texture = sixel_render_texture;
                    sixel_render_texture = tmp;
                }
            }

            // draw Framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            gl.viewport(
                rect.left() as i32,
                rect.top() as i32,
                rect.width() as i32,
                rect.height() as i32,
            );
            gl.use_program(Some(self.draw_program));
            gl.active_texture(glow::TEXTURE0);
            gl.uniform_1_i32(
                gl.get_uniform_location(self.draw_program, "u_render_texture")
                    .as_ref(),
                0,
            );
            gl.bind_texture(glow::TEXTURE_2D, Some(render_texture));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "u_effect")
                    .as_ref(),
                match self.post_processing {
                    crate::ui::main_window::PostProcessing::None => 0.0,
                    crate::ui::main_window::PostProcessing::CRT1 => 1.0,
                },
            );

            gl.uniform_2_f32(
                gl.get_uniform_location(self.draw_program, "u_resolution")
                    .as_ref(),
                rect.width(),
                rect.height(),
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.draw_program, "u_position")
                    .as_ref(),
                rect.left(),
                rect.top(),
            );

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.draw_arrays(glow::TRIANGLES, 3, 3);
        }
    }

    pub fn update_buffer(&mut self, gl: &glow::Context) {
        self.update_sixels(gl);

        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let cur_ms = since_the_epoch.as_millis();
        if self.caret_blink.update(cur_ms) || self.character_blink.update(cur_ms) {
            self.redraw_view = true;
        }

        if self.redraw_view {
            self.redraw_view = false;
            create_buffer_texture(gl, &self.buf, self.scroll_back_line, self.buffer_texture);
        }

        if self.redraw_palette || self.colors != self.buf.palette.colors.len() {
            self.redraw_palette = false;
            create_palette_texture(gl, &self.buf, self.palette_texture);
            self.colors = self.buf.palette.colors.len();
        }

        if self.redraw_font || self.fonts != self.buf.font_table.len() {
            self.redraw_font = false;
            create_font_texture(gl, &self.buf, self.palette_texture);
            self.fonts = self.buf.font_table.len();
        }
        let buf = &self.buf;
        let render_buffer_size = Vec2::new(
            buf.get_font_dimensions().width as f32 * buf.get_buffer_width() as f32,
            buf.get_font_dimensions().height as f32 * buf.get_buffer_height() as f32,
        );

        if render_buffer_size != self.render_buffer_size {
            unsafe {
                use glow::HasContext as _;
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
                gl.delete_texture(self.render_texture);
                gl.delete_texture(self.sixel_render_texture);

                let scale_filter = match self.scaling {
                    Scaling::Nearest => glow::NEAREST as i32,
                    Scaling::Linear => glow::LINEAR as i32,
                };
                let sixel_render_texture = gl.create_texture().unwrap();

                gl.bind_texture(glow::TEXTURE_2D, Some(sixel_render_texture));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    render_buffer_size.x as i32,
                    render_buffer_size.y as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    None,
                );
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, scale_filter);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, scale_filter);
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

                let render_texture = gl.create_texture().unwrap();
                gl.bind_texture(glow::TEXTURE_2D, Some(render_texture));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    render_buffer_size.x as i32,
                    render_buffer_size.y as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    None,
                );
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, scale_filter);
                gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, scale_filter);
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

                let depth_buffer = gl.create_renderbuffer().unwrap();
                gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth_buffer));
                gl.renderbuffer_storage(
                    glow::RENDERBUFFER,
                    glow::DEPTH_COMPONENT,
                    render_buffer_size.x as i32,
                    render_buffer_size.y as i32,
                );
                gl.framebuffer_renderbuffer(
                    glow::FRAMEBUFFER,
                    glow::DEPTH_ATTACHMENT,
                    glow::RENDERBUFFER,
                    Some(depth_buffer),
                );
                gl.framebuffer_texture(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    Some(render_texture),
                    0,
                );

                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                self.render_texture = render_texture;
                self.sixel_render_texture = sixel_render_texture;
                self.render_buffer_size = render_buffer_size;
            }
        }
    }
}

fn conv_color(c: u32, colors: u32) -> u8 {
    let r = ((255 * c) / colors) as u8;
    r
}

pub fn create_palette_texture(gl: &glow::Context, buf: &Buffer, palette_texture: NativeTexture) {
    let mut palette_data = Vec::new();
    for i in 0..buf.palette.colors.len() {
        let (r, g, b) = buf.palette.colors[i].get_rgb();
        palette_data.push(r);
        palette_data.push(g);
        palette_data.push(b);
        palette_data.push(0xFF);
    }
    unsafe {
        use glow::HasContext as _;
        gl.bind_texture(glow::TEXTURE_2D, Some(palette_texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as i32,
            buf.palette.colors.len() as i32,
            1,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&palette_data),
        );
    }
}

pub fn create_font_texture(gl: &glow::Context, buf: &Buffer, font_texture: NativeTexture) {
    let w = buf.font_table[0].size.width as usize;
    let h = buf.font_table[0].size.height as usize;

    let mut font_data = Vec::new();
    let chars_in_line = 16;
    let line_width = w * chars_in_line * 4;
    let height = h * 256 / chars_in_line;

    font_data.resize(line_width * height * buf.font_table.len(), 0);

    for i in 0..buf.font_table.len() {
        for ch in 0..256usize {
            let glyph = buf.font_table[i]
                .get_glyph(unsafe { char::from_u32_unchecked(ch as u32) })
                .unwrap();

            let x = ch % chars_in_line;
            let y = ch / chars_in_line;

            let offset = x * w * 4 + y * h * line_width + i * line_width * height;
            for y in 0..h {
                let scan_line = glyph.data[y];
                let mut po = offset + y * line_width;

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
    }

    unsafe {
        use glow::HasContext as _;
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(font_texture));
        gl.tex_image_3d(
            glow::TEXTURE_2D_ARRAY,
            0,
            glow::RGB as i32,
            line_width as i32 / 4,
            height as i32,
            buf.font_table.len() as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&font_data),
        );
    }
}

pub fn create_buffer_texture(
    gl: &glow::Context,
    buf: &Buffer,
    scroll_back_line: i32,
    buffer_texture: NativeTexture,
) {
    let first_line = max(
        0,
        buf.layers[0].lines.len() as i32 - buf.get_buffer_height(),
    );
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
                buffer_data.push(' ' as u8);
            } else {
                buffer_data.push(ch.ch as u8);
            }
            if ch.attribute.is_bold() {
                buffer_data.push(conv_color(ch.attribute.get_foreground() + 8, colors));
            } else {
                buffer_data.push(conv_color(ch.attribute.get_foreground(), colors));
            }
            buffer_data.push(conv_color(ch.attribute.get_background(), colors));
            if buf.font_table.len() > 0 {
                buffer_data.push(
                    (255.0 * ch.get_font_page() as f32 / (buf.font_table.len() - 1) as f32) as u8,
                );
            } else {
                buffer_data.push(0);
            }
        }

        if is_double_height {
            for x in 0..buf.get_buffer_width() {
                let ch = buf
                    .get_char_xy(x, first_line - scroll_back_line + y)
                    .unwrap_or_default();

                if !ch.attribute.is_double_height() {
                    buffer_data.push(' ' as u8);
                } else {
                    buffer_data.push(ch.ch as u8);
                }

                if ch.attribute.is_bold() {
                    buffer_data.push(conv_color(ch.attribute.get_foreground() + 8, colors));
                } else {
                    buffer_data.push(conv_color(ch.attribute.get_foreground(), colors));
                }

                buffer_data.push(conv_color(ch.attribute.get_background(), colors));

                if buf.font_table.len() > 0 {
                    buffer_data.push(
                        (255.0 * ch.get_font_page() as f32 / (buf.font_table.len() - 1) as f32)
                            as u8,
                    );
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
            } else if ch.attribute.is_underlined() {
                1
            } else {
                0
            };
            if ch.attribute.is_crossed_out() {
                attr |= 4
            }

            if ch.attribute.is_double_height() {
                is_double_height = true;
                attr |= 8
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
                } else if ch.attribute.is_underlined() {
                    1
                } else {
                    0
                };
                if ch.attribute.is_crossed_out() {
                    attr |= 4
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
        use glow::HasContext as _;
        gl.active_texture(glow::TEXTURE0 + 4);

        gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(buffer_texture));
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
