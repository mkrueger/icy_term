#![allow(clippy::many_single_char_names, clippy::collapsible_match)]
use glow::{HasContext, NativeTexture};
use icy_engine::{Position, SixelReadStatus};

use super::ViewState;

pub struct SixelCacheEntry {
    pub status: SixelReadStatus,
    pub old_line: i32,
    pub data_opt: Option<Vec<u8>>,

    pub pos: Position,
    pub size: icy_engine::Size<i32>,

    pub texture_opt: Option<NativeTexture>,
}

impl SixelCacheEntry {
    pub fn rect(&self) -> icy_engine::Rectangle {
        icy_engine::Rectangle {
            start: self.pos,
            size: self.size,
        }
    }

    fn reset(&mut self) {
        self.status = SixelReadStatus::default();
        self.old_line = 0;
        self.data_opt = None;
        self.texture_opt = None;
    }
}

impl ViewState {
    pub fn update_sixels(&mut self, gl: &glow::Context) -> bool {
        let buffer = &mut self.buf;
        let sixel_len = buffer.layers[0].sixels.len();
        if sixel_len == 0 {
            for sx in &self.sixel_cache {
                if let Some(tex) = sx.texture_opt {
                    unsafe {
                        gl.delete_texture(tex);
                    }
                }
            }
            self.sixel_cache.clear();
        }

        let mut res = false;
        let mut i = 0;
        while i < sixel_len {
            let reset_entry = matches!(
                buffer.layers[0].sixels[i].read_status,
                SixelReadStatus::Updated
            );
            if reset_entry {
                buffer.layers[0].sixels[i].read_status = SixelReadStatus::Finished;
            }

            let sixel = &buffer.layers[0].sixels[i];

            if sixel.width() == 0 || sixel.height() == 0 {
                i += 1;
                continue;
            }

            let mut old_line = 0;
            let current_line = match sixel.read_status {
                SixelReadStatus::Position(_, y) => y * 6,
                SixelReadStatus::Error | SixelReadStatus::Finished | SixelReadStatus::Updated => {
                    sixel.height() as i32
                }
                SixelReadStatus::NotStarted => 0,
            };

            if let Some(entry) = self.sixel_cache.get_mut(i) {
                if reset_entry {
                    if let Some(texture) = entry.texture_opt {
                        unsafe {
                            gl.delete_texture(texture);
                        }
                    }
                    entry.reset();
                }

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
                    vec![0; data_len]
                }
            } else {
                vec![0; data_len]
            };

            let mut j = old_line as usize * sixel.width() as usize * 4;

            for y in old_line..current_line {
                for x in 0..sixel.width() {
                    let column = &sixel.picture[x as usize];
                    let data = if let Some(col) = column.get(y as usize) {
                        if let Some(c) = col {
                            let (r, g, b) = c.get_rgb();
                            [r, g, b, 0xFF]
                        } else {
                            [0, 0, 0, 0]
                        }
                    } else {
                        [0, 0, 0, 0]
                    };
                    if j >= v.len() {
                        v.extend_from_slice(&data);
                    } else {
                        v[j] = data[0];
                        v[j + 1] = data[1];
                        v[j + 2] = data[2];
                        v[j + 3] = data[3];
                    }
                    j += 4;
                }
            }
            let (texture_opt, data_opt, clear) = match sixel.read_status {
                SixelReadStatus::Finished | SixelReadStatus::Error => unsafe {
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
                        Some(&v),
                    );
                    (Some(texture), None, true)
                },
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
                texture_opt,
            };

            if removed_index < 0 {
                self.sixel_cache.push(new_entry);
                if clear {
                    self.clear_invisible_sixel_cache(gl, self.sixel_cache.len() - 1);
                    break;
                }
            } else {
                self.sixel_cache.insert(removed_index as usize, new_entry);
                if clear {
                    self.clear_invisible_sixel_cache(gl, removed_index as usize);
                    break;
                }
            }
        }
        res
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
                    if let Some(texture) = &t.texture_opt {
                        unsafe {
                            gl.delete_texture(*texture);
                        }
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
