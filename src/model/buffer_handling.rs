use super::{Layer, Position, DosChar, Size, Palette, Line, BitFont };

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BufferType {
    LegacyDos  = 0b_0000,  // 0-15 fg, 0-7 bg, blink
    LegacyIce  = 0b_0001,  // 0-15 fg, 0-15 bg
    ExtFont    = 0b_0010,  // 0-7 fg, 0-7 bg, blink + extended font
    ExtFontIce = 0b_0011,  // 0-7 fg, 0-15 bg + extended font
    NoLimits   = 0b_0111   // free colors, blink + extended font 
}

impl BufferType {
    pub fn use_ice_colors(self) -> bool {
        self == BufferType::LegacyIce || self == BufferType::ExtFontIce
    }

    pub fn use_blink(self) -> bool {
        self == BufferType::LegacyDos || self == BufferType::ExtFont || self == BufferType::NoLimits
    } 
    
    pub fn use_extended_font(self) -> bool {
        self == BufferType::ExtFont || self == BufferType::ExtFontIce
    }
/* 
    pub fn get_fg_colors(self) -> u8 {
        match self {
            BufferType::LegacyDos |
            BufferType::LegacyIce |
            BufferType::NoLimits => 16, // may change in the future

            BufferType::ExtFont |
            BufferType::ExtFontIce => 8,
        }
    }

    pub fn get_bg_colors(self) -> u8 {
        match self {
            BufferType::LegacyDos |
            BufferType::ExtFont => 8,
            
            BufferType::LegacyIce |
            BufferType::ExtFontIce |
            BufferType::NoLimits => 16 // may change in the future
        }
    }*/
}

pub struct Buffer {
    pub width: u16,
    pub height: u16,

    pub buffer_type: BufferType,

    pub palette: Palette,

    pub font: BitFont,
    pub extended_font: Option<BitFont>,
    
    pub layer: Layer
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer").field("width", &self.width).field("height", &self.height).field("custom_palette", &self.palette).field("font", &self.font).finish()
    }
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            width: 80,
            height: 25,

            buffer_type: BufferType::LegacyDos,

            palette: Palette::new(),
            layer: Layer::new(),
            font: BitFont::default(),
            extended_font: None,
        }
    }

    pub fn create(width: u16, height: u16) -> Self {
        let mut res = Buffer::new();
        res.width = width;
        res.height = height;
        res.layer.lines.resize(height as usize, Line::create(width));

        res
    }

    pub fn clear(&mut self) {
        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                self.set_char(Position::from(x, y), Some(DosChar::new()));
            }
        }
    }

    pub fn get_font_scanline(&self, ext: bool, ch: u16, y: usize) -> u32
    {
        if ext { 
            self.extended_font.as_ref().unwrap().get_scanline(ch, y)
        } else { 
            self.font.get_scanline(ch, y)
        }
    }

    pub fn get_font_dimensions(&self) -> Size<u8>
    {
        self.font.size
    }

    pub fn set_char(&mut self, pos: Position, dos_char: Option<DosChar>) {
        self.layer.set_char(pos, dos_char);
    }

    pub fn get_char(&self, pos: Position) ->  Option<DosChar> {
        self.layer.get_char(pos)
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer::new()
    }
}
