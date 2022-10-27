#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8
}

impl Color {
    pub fn get_rgb_f32(self) -> (f32, f32, f32) {
        (
            self.r as f32 / 255_f32,
            self.g as f32 / 255_f32,
            self.b as f32 / 255_f32
        )
    }
    
}
impl PartialEq for Color {
    fn eq(&self, other: &Color) -> bool {
        self.r == other.r && self.g == other.g && self.b == other.b
    }
}


#[derive(Debug, Clone)]
pub struct Palette {
    pub colors: Vec<Color>
}

pub const DOS_DEFAULT_PALETTE: [Color; 16] = [
    Color { r: 0x00, g: 0x00, b: 0x00 }, // black
    Color { r: 0x00, g: 0x00, b: 0xAA }, // blue
    Color { r: 0x00, g: 0xAA, b: 0x00 }, // green
    Color { r: 0x00, g: 0xAA, b: 0xAA }, // cyan
    Color { r: 0xAA, g: 0x00, b: 0x00 }, // red
    Color { r: 0xAA, g: 0x00, b: 0xAA }, // magenta
    Color { r: 0xAA, g: 0x55, b: 0x00 }, // brown
    Color { r: 0xAA, g: 0xAA, b: 0xAA }, // lightgray
    Color { r: 0x55, g: 0x55, b: 0x55 }, // darkgray
    Color { r: 0x55, g: 0x55, b: 0xFF }, // lightblue
    Color { r: 0x55, g: 0xFF, b: 0x55 }, // lightgreen
    Color { r: 0x55, g: 0xFF, b: 0xFF }, // lightcyan
    Color { r: 0xFF, g: 0x55, b: 0x55 }, // lightred
    Color { r: 0xFF, g: 0x55, b: 0xFF }, // lightmagenta
    Color { r: 0xFF, g: 0xFF, b: 0x55 }, // yellow
    Color { r: 0xFF, g: 0xFF, b: 0xFF }, // white
];

impl Palette {
    pub fn new() -> Self {
        Palette { colors: DOS_DEFAULT_PALETTE.to_vec() }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}