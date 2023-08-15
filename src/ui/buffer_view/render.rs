use eframe::epaint::{PaintCallbackInfo, Rect, Vec2};

use crate::ui::{Scaling, MONO_COLORS};

use super::ViewState;

impl ViewState {
    pub fn paint(&self, gl: &glow::Context, info: &PaintCallbackInfo, rect: Rect) {
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

            self.terminal_renderer.render_terminal(gl, self);

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.draw_arrays(glow::TRIANGLES, 3, 3);

            // draw sixels
            let mut render_texture = self.render_texture;
            let mut sixel_render_texture = self.sixel_render_texture;

            for sixel in &self.sixel_cache {
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
                gl.bind_texture(glow::TEXTURE_2D, Some(sixel.texture));

                gl.uniform_2_f32(
                    gl.get_uniform_location(self.sixel_shader, "u_resolution")
                        .as_ref(),
                    self.render_buffer_size.x,
                    self.render_buffer_size.y,
                );

                let x = sixel.pos.x as f32 * self.buf.get_font_dimensions().width as f32;
                let y = sixel.pos.y as f32 * self.buf.get_font_dimensions().height as f32;

                let w = sixel.size.width as f32 * sixel.x_scale as f32;
                let h = sixel.size.height as f32 * sixel.y_scale as f32;

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

                std::mem::swap(&mut render_texture, &mut sixel_render_texture);
            }

            // draw Framebuffer
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            gl.viewport(
                info.clip_rect.left() as i32,
                (info.screen_size_px[1] as f32 - info.clip_rect.max.y * info.pixels_per_point)
                    as i32,
                (info.viewport.width() * info.pixels_per_point) as i32,
                (info.viewport.height() * info.pixels_per_point) as i32,
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
                if self.monitor_settings.use_filter {
                    10.0
                } else {
                    0.0
                },
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "u_use_monochrome")
                    .as_ref(),
                if self.monitor_settings.monitor_type > 0 {
                    1.0
                } else {
                    0.0
                },
            );

            if self.monitor_settings.monitor_type > 0 {
                let r = MONO_COLORS[self.monitor_settings.monitor_type - 1].0 as f32 / 255.0;
                let g = MONO_COLORS[self.monitor_settings.monitor_type - 1].1 as f32 / 255.0;
                let b = MONO_COLORS[self.monitor_settings.monitor_type - 1].2 as f32 / 255.0;
                gl.uniform_3_f32(
                    gl.get_uniform_location(self.draw_program, "u_monchrome_mask")
                        .as_ref(),
                    r,
                    g,
                    b,
                );
            }

            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "gamma").as_ref(),
                self.monitor_settings.gamma / 50.0,
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "contrast")
                    .as_ref(),
                self.monitor_settings.contrast / 50.0,
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "saturation")
                    .as_ref(),
                self.monitor_settings.saturation / 50.0,
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "brightness")
                    .as_ref(),
                self.monitor_settings.brightness / 30.0,
            );
            /*
                        gl.uniform_1_f32(
                            gl.get_uniform_location(self.draw_program, "light")
                                .as_ref(),
                                self.light);
            */
            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "blur").as_ref(),
                self.monitor_settings.blur / 30.0,
            );

            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "curvature")
                    .as_ref(),
                self.monitor_settings.curvature / 30.0,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.draw_program, "u_scanlines")
                    .as_ref(),
                0.5 * (self.monitor_settings.scanlines / 100.0),
            );

            gl.uniform_2_f32(
                gl.get_uniform_location(self.draw_program, "u_resolution")
                    .as_ref(),
                rect.width() * info.pixels_per_point,
                rect.height() * info.pixels_per_point,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.draw_program, "u_position")
                    .as_ref(),
                rect.left() * info.pixels_per_point,
                rect.top() * info.pixels_per_point,
            );

            gl.uniform_4_f32(
                gl.get_uniform_location(self.draw_program, "u_draw_rect")
                    .as_ref(),
                info.clip_rect.left() * info.pixels_per_point,
                info.clip_rect.top() * info.pixels_per_point,
                info.clip_rect.width() * info.pixels_per_point,
                info.clip_rect.height() * info.pixels_per_point,
            );

            gl.uniform_4_f32(
                gl.get_uniform_location(self.draw_program, "u_draw_area")
                    .as_ref(),
                (rect.left() - 3.) * info.pixels_per_point,
                (rect.top() - info.clip_rect.top() - 4.) * info.pixels_per_point,
                (rect.right() - 3.) * info.pixels_per_point,
                (rect.bottom() - info.clip_rect.top() - 4.) * info.pixels_per_point,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.draw_program, "u_size")
                    .as_ref(),
                rect.width() * info.pixels_per_point,
                rect.height() * info.pixels_per_point,
            );

            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.draw_arrays(glow::TRIANGLES, 3, 3);
        }
    }

    pub fn update_buffer(&mut self, gl: &glow::Context) {
        self.update_sixels(gl);
        self.buf.layers[0].updated_sixels = false;

        self.terminal_renderer
            .update_textures(gl, &mut self.buf, self.scroll_back_line);

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
