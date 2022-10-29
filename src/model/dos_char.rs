use super::TextAttribute;

#[derive(Clone, Copy, Debug)]
pub struct DosChar {
    pub char_code: u16,
    pub attribute: TextAttribute,
    pub ext_font: bool,
}

impl Default for DosChar {
    fn default() -> Self {
        DosChar::new()
    }
}

impl DosChar {
    pub fn new() -> Self {
        DosChar {
            char_code: b' ' as u16,
            attribute: super::TextAttribute::DEFAULT,
            ext_font: false
        }
    }   
    
    pub fn from(char_code: u16, attribute: TextAttribute) -> Self {
        DosChar {
            char_code,
            attribute,
            ext_font: false
        }
    }

    pub fn _use_extended_font(&self) -> bool {
        self.ext_font
    }
}


impl PartialEq for DosChar {
    fn eq(&self, other: &DosChar) -> bool {
        self.char_code == other.char_code && self.attribute == other.attribute
    }
}

impl std::fmt::Display for DosChar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(Char: {}/0x{0:X} '{}', Attr: {})", self.char_code, char::from_u32(self.char_code as u32).unwrap(),  self.attribute)
    }
}
 /*
pub fn get_color(color: u8) -> &'static str
{
    match color {
        0 => "Black",
        1 => "Blue",
        2 => "Green",
        3 => "Aqua",
        4 => "Red",
        5 => "Purple",
        6 => "Brown",
        7 => "Light Gray",
        8 => "Gray",
        9 => "Light Blue",
        10 => "Light Green",
        11 => "Light Aqua",
        12 => "Light Red",
        13 => "Light Purple",
        14 => "Light Yelllow",
        15 => "White",
        _ => "Unknown"
    }
}
*/