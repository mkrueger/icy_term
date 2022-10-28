use super::BufferType;


#[derive(Clone, Copy, Debug, Default)]
pub struct TextAttribute {
    foreground_color: u8,
    background_color: u8,
    blink: bool
}

impl std::fmt::Display for TextAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Attr: {:X}, fg {}, bg {}, blink {})", self.as_u8(BufferType::LegacyDos), self.get_foreground(), self.get_background(), self.is_blink())
    }
}

impl TextAttribute
{
    pub const DEFAULT : TextAttribute = TextAttribute{ foreground_color: 7, background_color: 0, blink: false };

    pub fn as_u8(self, buffer_type: BufferType) -> u8
    {
        let fg = if buffer_type.use_extended_font() {
            self.foreground_color & 0b_0111
        } else {
            self.foreground_color & 0b_1111
        };

        let bg = if buffer_type.use_blink() {
            self.background_color & 0b_0111 | if self.is_blink() { 0b_1000 } else { 0 }
        } else {
            self.background_color & 0b_0111
        };

        fg | bg << 4
    }

    pub fn set_foreground_bold(&mut self, is_bold: bool)
    {
        if self.foreground_color < 16  {
            if is_bold {
                self.foreground_color |= 0b0000_1000;
            } else {
                self.foreground_color &= 0b1111_0111;
            }
        }
    }

    pub fn set_background_bold(&mut self, is_bold: bool)
    {
        if self.background_color < 16  {
            if is_bold {
                self.background_color |= 0b0000_1000;
            } else {
                self.background_color &= 0b1111_0111;
            }
        }
    }

    pub fn is_blink(self) -> bool
    {
        self.blink
    }

    pub fn set_blink(&mut self, is_blink: bool)
    {
        self.blink = is_blink;
    }

    pub fn get_foreground(self) -> u8
    {
        self.foreground_color
    }

    pub fn set_foreground(&mut self, color: u8) 
    {
        self.foreground_color = color;
    }

    pub fn set_foreground_without_bold(&mut self, color: u8) 
    {
        assert!(color < 0b1000);
        if self.foreground_color < 16  {
            self.foreground_color = (0b1000 & self.foreground_color) | color;
        }
    }

    pub fn set_background_without_bold(&mut self, color: u8) 
    {
        assert!(color < 0b1000);
        if self.background_color < 16  {
            self.background_color = (0b1000 & self.background_color) | color;
        }
    }

    pub fn get_background(self) -> u8
    {
        self.background_color
    }
}

impl PartialEq for TextAttribute {
    fn eq(&self, other: &TextAttribute) -> bool {
        self.foreground_color == other.foreground_color && self.background_color == other.background_color
    }
}
