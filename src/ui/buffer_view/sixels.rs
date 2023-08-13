#![allow(clippy::many_single_char_names, clippy::collapsible_match)]
use glow::{HasContext, NativeTexture};
use icy_engine::{Position};

use super::ViewState;

pub struct SixelCacheEntry {
    pub pos: Position,
    pub size: icy_engine::Size<i32>,
    pub x_scale: i32,
    pub y_scale: i32,

    pub texture: NativeTexture,
}

impl SixelCacheEntry {
    pub fn rect(&self) -> icy_engine::Rectangle {
        icy_engine::Rectangle {
            start: self.pos,
            size: self.size,
        }
    }

    fn reset(&mut self) {
    }
}

impl ViewState {
    pub fn update_sixels(&mut self, gl: &glow::Context) -> bool {
        let buffer = &mut self.buf;
        if buffer.layers[0].sixels.len() == 0 {
            for sx in &self.sixel_cache {
                unsafe {
                    gl.delete_texture(sx.texture);
                }
            }
            self.sixel_cache.clear();
            return false;
        }
        if !buffer.layers[0].updated_sixels {
            return false;
        }
        for sx in &self.sixel_cache {
            unsafe {
                gl.delete_texture(sx.texture);
            }
        }
        self.sixel_cache.clear();

        let sixel_len = buffer.layers[0].sixels.len();
     //   if sixel_len == 0 {
       //     return false;
      //  }
        let mut res = false;
        let mut i = 0;
        while i < sixel_len {
            let sixel = &buffer.layers[0].sixels[i];
            let data_len = (sixel.height() * sixel.width() * 4) as usize;
            let mut data = Vec::new();

            println!("get sixel data!!! {}x{}", sixel.width(), sixel.height());
            for y in 0..sixel.height() {
                data.extend(&sixel.picture_data[y as usize]);
            }

            println!("gen texture");

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
                    glow::RGB as i32,
                    sixel.width() as i32,
                    sixel.height() as i32,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(&data),
                );

                let new_entry = SixelCacheEntry {
                    pos: sixel.position,
                    x_scale: sixel.horizontal_scale,
                    y_scale: sixel.vertical_scale,
                    size: icy_engine::Size {
                        width: sixel.width() as i32,
                        height: sixel.height() as i32,
                    },
                    texture
                };
                self.sixel_cache.push(new_entry);
            }
            i+=1;
        }

        res
    }

    fn clear_sixel_cache(&mut self, gl: &glow::Context) {
    }

    pub fn clear_invisible_sixel_cache(&mut self, gl: &glow::Context, j: usize) {
        // remove cache entries that are removed by the engine
        if j > 0 {
            let cur_rect = self.sixel_cache[j].rect();
            let mut i = j - 1;
            loop {
                let other_rect = self.sixel_cache[i].rect();
                if cur_rect.contains(other_rect) {
                    let t = self.sixel_cache.remove(i);
                    unsafe {
                        gl.delete_texture(t.texture);
                    }
                    self.buf.layers[0].sixels.remove(i);
                }
                if i == 0 {
                    break;
                }
                i -= 1;
            }
        }
    }
}
