use egui::PaintCallbackInfo;
use egui::Rect;
use egui::Vec2;
use glow::HasContext as _;
use glow::Texture;
use icy_engine::Buffer;

use crate::prepare_shader;
use crate::ui::buffer_view::SHADER_SOURCE;
use crate::MonitorSettings;
use crate::MONO_COLORS;

pub struct OutputRenderer {
    output_shader: glow::Program,

    pub framebuffer: glow::Framebuffer,
    pub render_texture: glow::Texture,
    pub render_buffer_size: Vec2,
    pub vertex_array: glow::VertexArray,
}

impl OutputRenderer {
    pub fn new(gl: &glow::Context, buf: &Buffer, filter: i32) -> Self {
        unsafe {
            let render_buffer_size = Vec2::new(
                buf.get_font_dimensions().width as f32 * buf.get_buffer_width() as f32,
                buf.get_font_dimensions().height as f32 * buf.get_buffer_height() as f32,
            );

            let output_shader = compile_output_shader(gl);
            let framebuffer = gl.create_framebuffer().unwrap();
            let render_texture = create_screen_render_texture(gl, render_buffer_size, filter);
            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            Self {
                output_shader,
                framebuffer,
                render_texture,
                render_buffer_size,
                vertex_array,
            }
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.output_shader);
            gl.delete_vertex_array(self.vertex_array);
            gl.delete_texture(self.render_texture);
            gl.delete_framebuffer(self.framebuffer);
        }
    }

    pub(crate) unsafe fn init_output(&self, gl: &glow::Context) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
        gl.bind_texture(glow::TEXTURE_2D, Some(self.render_texture));
        gl.viewport(
            0,
            0,
            self.render_buffer_size.x as i32,
            self.render_buffer_size.y as i32,
        );
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(self.render_texture),
            0,
        );

        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.clear_color(0., 0., 0., 1.0);
        crate::check_gl_error!(gl, "init_output");
    }

    pub unsafe fn render_to_screen(
        &self,
        gl: &glow::Context,
        info: &PaintCallbackInfo,
        output_texture: glow::Texture,
        buffer_rect: egui::Rect,
        terminal_rect: Rect,
        monitor_settings: &MonitorSettings,
    ) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.viewport(
            info.clip_rect.left() as i32,
            (info.screen_size_px[1] as f32 - info.clip_rect.max.y * info.pixels_per_point) as i32,
            (info.viewport.width() * info.pixels_per_point) as i32,
            (info.viewport.height() * info.pixels_per_point) as i32,
        );
        gl.scissor(
            info.clip_rect.left() as i32,
            (info.screen_size_px[1] as f32 - info.clip_rect.max.y * info.pixels_per_point) as i32,
            (info.viewport.width() * info.pixels_per_point) as i32,
            (info.viewport.height() * info.pixels_per_point) as i32,
        );

        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
        gl.use_program(Some(self.output_shader));
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(output_texture));

        gl.uniform_1_i32(
            gl.get_uniform_location(self.output_shader, "u_render_texture")
                .as_ref(),
            0,
        );
        gl.bind_texture(glow::TEXTURE_2D, Some(output_texture));

        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "u_effect")
                .as_ref(),
            if monitor_settings.use_filter {
                10.0
            } else {
                0.0
            },
        );

        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "u_use_monochrome")
                .as_ref(),
            if monitor_settings.monitor_type > 0 {
                1.0
            } else {
                0.0
            },
        );

        if monitor_settings.monitor_type > 0 {
            let r = MONO_COLORS[monitor_settings.monitor_type - 1].0 as f32 / 255.0;
            let g = MONO_COLORS[monitor_settings.monitor_type - 1].1 as f32 / 255.0;
            let b = MONO_COLORS[monitor_settings.monitor_type - 1].2 as f32 / 255.0;
            gl.uniform_3_f32(
                gl.get_uniform_location(self.output_shader, "u_monchrome_mask")
                    .as_ref(),
                r,
                g,
                b,
            );
        }

        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "gamma")
                .as_ref(),
            monitor_settings.gamma / 50.0,
        );

        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "contrast")
                .as_ref(),
            monitor_settings.contrast / 50.0,
        );

        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "saturation")
                .as_ref(),
            monitor_settings.saturation / 50.0,
        );

        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "brightness")
                .as_ref(),
            monitor_settings.brightness / 30.0,
        );
        /*
                    gl.uniform_1_f32(
                        gl.get_uniform_location(self.draw_program, "light")
                            .as_ref(),
                            self.light);
        */
        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "blur").as_ref(),
            monitor_settings.blur / 30.0,
        );

        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "curvature")
                .as_ref(),
            monitor_settings.curvature / 30.0,
        );
        gl.uniform_1_f32(
            gl.get_uniform_location(self.output_shader, "u_scanlines")
                .as_ref(),
            0.5 * (monitor_settings.scanlines / 100.0),
        );

        gl.uniform_2_f32(
            gl.get_uniform_location(self.output_shader, "u_resolution")
                .as_ref(),
            terminal_rect.width() * info.pixels_per_point,
            terminal_rect.height() * info.pixels_per_point,
        );

        gl.uniform_4_f32(
            gl.get_uniform_location(self.output_shader, "u_buffer_rect")
                .as_ref(),
            buffer_rect.left() / terminal_rect.width(),
            buffer_rect.top() / terminal_rect.height(),
            buffer_rect.right() / terminal_rect.width(),
            buffer_rect.bottom() / terminal_rect.height(),
        );

        gl.bind_vertex_array(Some(self.vertex_array));
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
    }

    pub(crate) fn update_render_buffer(
        &mut self,
        gl: &glow::Context,
        buf: &Buffer,
        scale_filter: i32,
    ) {
        let render_buffer_size = Vec2::new(
            buf.get_font_dimensions().width as f32 * buf.get_buffer_width() as f32,
            buf.get_font_dimensions().height as f32 * buf.get_buffer_height() as f32,
        );
        if render_buffer_size == self.render_buffer_size {
            return;
        }
        unsafe {
            use glow::HasContext as _;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            gl.delete_texture(self.render_texture);

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

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            self.render_texture = render_texture;
            self.render_buffer_size = render_buffer_size;
        }
    }
}

unsafe fn compile_output_shader(gl: &glow::Context) -> glow::Program {
    let draw_program = gl.create_program().expect("Cannot create program");
    let (vertex_shader_source, fragment_shader_source) = (
        prepare_shader!(SHADER_SOURCE),
        prepare_shader!(include_str!("output_renderer.shader.frag")),
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
            gl.attach_shader(draw_program, shader);
            shader
        })
        .collect();

    gl.link_program(draw_program);
    assert!(
        gl.get_program_link_status(draw_program),
        "{}",
        gl.get_program_info_log(draw_program)
    );

    for shader in shaders {
        gl.detach_shader(draw_program, shader);
        gl.delete_shader(shader);
    }
    draw_program
}

unsafe fn create_screen_render_texture(
    gl: &glow::Context,
    render_buffer_size: Vec2,
    filter: i32,
) -> Texture {
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

    render_texture
}
