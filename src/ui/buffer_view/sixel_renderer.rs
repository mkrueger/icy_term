use egui::Vec2;
use glow::HasContext as _;
use icy_engine::Buffer;
use icy_engine::Position;

use crate::prepare_shader;
use crate::ui::buffer_view::SHADER_SOURCE;

use super::output_renderer::OutputRenderer;
use super::BufferView;

pub struct SixelRenderer {
    sixel_cache: Vec<SixelCacheEntry>,
    sixel_shader: glow::Program,
    sixel_render_texture: glow::Texture,
    render_buffer_size: Vec2,
}

impl SixelRenderer {
    pub fn new(gl: &glow::Context, buf: &Buffer, filter: i32) -> Self {
        unsafe {
            let sixel_shader = compile_shader(gl);
            let sixel_render_texture = create_sixel_render_texture(gl, buf, filter);

            Self {
                sixel_cache: Vec::new(),
                sixel_shader,
                sixel_render_texture,
                render_buffer_size: Vec2::ZERO,
            }
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.sixel_shader);
            gl.delete_texture(self.sixel_render_texture);
        }
    }

    pub unsafe fn render_sixels(
        &self,
        gl: &glow::Context,
        buffer_view: &BufferView,
        output_renderer: &OutputRenderer,
    ) -> glow::Texture {
        let mut render_texture = output_renderer.render_texture;
        let mut sixel_render_texture = self.sixel_render_texture;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(output_renderer.framebuffer));

        for sixel in &self.sixel_cache {
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(sixel_render_texture),
                0,
            );
            gl.bind_texture(glow::TEXTURE_2D, Some(render_texture));
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
            gl.uniform_2_f32(
                gl.get_uniform_location(self.sixel_shader, "u_sixel_scale")
                    .as_ref(),
                sixel.x_scale as f32,
                sixel.y_scale as f32,
            );

            let fontdim: icy_engine::Size<u8> = buffer_view.buf.get_font_dimensions();
            let fh: f32 = fontdim.height as f32;

            let x = sixel.pos.x as f32 * buffer_view.buf.get_font_dimensions().width as f32;
            let y = sixel.pos.y as f32 * buffer_view.buf.get_font_dimensions().height as f32
                - (buffer_view.viewport_top / buffer_view.char_size.y * fh);

            let w = sixel.size.width as f32;
            let h = sixel.size.height as f32;
            gl.uniform_4_f32(
                gl.get_uniform_location(self.sixel_shader, "u_sixel_rectangle")
                    .as_ref(),
                x / output_renderer.render_buffer_size.x,
                y / (output_renderer.render_buffer_size.y),
                (x + w) / output_renderer.render_buffer_size.x,
                (y + h) / (output_renderer.render_buffer_size.y),
            );

            gl.bind_vertex_array(Some(output_renderer.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            std::mem::swap(&mut render_texture, &mut sixel_render_texture);
        }
        crate::check_gl_error!(gl, "render_sixels");

        render_texture
    }

    pub fn update_sixels(&mut self, gl: &glow::Context, buf: &mut Buffer, scale_filter: i32) {
        let render_buffer_size = Vec2::new(
            buf.get_font_dimensions().width as f32 * buf.get_buffer_width() as f32,
            buf.get_font_dimensions().height as f32 * buf.get_buffer_height() as f32,
        );
        if render_buffer_size != self.render_buffer_size {
            self.render_buffer_size = render_buffer_size;
            unsafe {
                self.create_sixel_render_texture(gl, render_buffer_size, scale_filter);
            }
        }

        let sixels_updated = buf.update_sixel_threads();
        if buf.layers[0].sixels.is_empty() {
            for sx in &self.sixel_cache {
                unsafe {
                    gl.delete_texture(sx.texture);
                }
            }
            self.sixel_cache.clear();
            return;
        }
        if !sixels_updated {
            return;
        }
        for sx in &self.sixel_cache {
            unsafe {
                gl.delete_texture(sx.texture);
            }
        }
        self.sixel_cache.clear();

        for sixel in &buf.layers[0].sixels {
            unsafe {
                let texture = gl.create_texture().unwrap();
                gl.active_texture(glow::TEXTURE0 + 6);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
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
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as i32,
                    sixel.width() as i32,
                    sixel.height() as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(&sixel.picture_data),
                );

                let new_entry = SixelCacheEntry {
                    pos: sixel.position,
                    x_scale: sixel.horizontal_scale,
                    y_scale: sixel.vertical_scale,
                    size: icy_engine::Size {
                        width: sixel.width() as i32,
                        height: sixel.height() as i32,
                    },
                    texture,
                };
                self.sixel_cache.push(new_entry);
            }
        }
        crate::check_gl_error!(gl, "update_sixels");
    }

    pub(crate) unsafe fn create_sixel_render_texture(
        &mut self,
        gl: &glow::Context,
        render_buffer_size: Vec2,
        scale_filter: i32,
    ) {
        gl.delete_texture(self.sixel_render_texture);

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
        crate::check_gl_error!(gl, "create_sixel_render_texture 1");

        self.sixel_render_texture = sixel_render_texture;
    }
}

pub struct SixelCacheEntry {
    pub pos: Position,
    pub size: icy_engine::Size<i32>,
    pub x_scale: i32,
    pub y_scale: i32,

    pub texture: glow::Texture,
}

unsafe fn create_sixel_render_texture(
    gl: &glow::Context,
    buf: &Buffer,
    filter: i32,
) -> glow::Texture {
    let sixel_render_texture = gl.create_texture().unwrap();
    let render_buffer_size = Vec2::new(
        buf.get_font_dimensions().width as f32 * buf.get_buffer_width() as f32,
        buf.get_font_dimensions().height as f32 * buf.get_buffer_height() as f32,
    );

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
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, filter);
    gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, filter);
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
    crate::check_gl_error!(gl, "create_sixel_render_texture");

    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    sixel_render_texture
}

unsafe fn compile_shader(gl: &glow::Context) -> glow::Program {
    let sixel_shader = gl.create_program().expect("Cannot create program");
    let (vertex_shader_source, fragment_shader_source) = (
        prepare_shader!(SHADER_SOURCE),
        prepare_shader!(include_str!("sixel_renderer.shader.frag")),
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
            gl.attach_shader(sixel_shader, shader);
            shader
        })
        .collect();

    gl.link_program(sixel_shader);
    assert!(
        gl.get_program_link_status(sixel_shader),
        "{}",
        gl.get_program_info_log(sixel_shader)
    );

    for shader in shaders {
        gl.detach_shader(sixel_shader, shader);
        gl.delete_shader(shader);
    }
    crate::check_gl_error!(gl, "compile_shader");

    sixel_shader
}
