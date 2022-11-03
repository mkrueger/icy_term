use std::fmt::Display;

use icy_engine::{Palette, BitFont, PETSCIIParser, C64_DEFAULT_PALETTE, AvatarParser, AnsiParser};

use super::BufferView;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScreenMode {
    DOS(i32, i32),
    C64,
    C128(i32),
    Atari,
    AtariXep80
}

pub const DEFAULT_MODES: [ScreenMode; 20] = [
    ScreenMode::DOS(80, 25),
    ScreenMode::DOS(80, 28),
    ScreenMode::DOS(80, 30),
    ScreenMode::DOS(80, 43),
    ScreenMode::DOS(80, 50),
    ScreenMode::DOS(80, 60),
    ScreenMode::DOS(132, 37),
    ScreenMode::DOS(132, 52),
    ScreenMode::DOS(132, 25),
    ScreenMode::DOS(132, 28),
    ScreenMode::DOS(132, 30),
    ScreenMode::DOS(132, 34),
    ScreenMode::DOS(132, 43),
    ScreenMode::DOS(132, 50),
    ScreenMode::DOS(132, 60),
    ScreenMode::C64,
    ScreenMode::C128(40),
    ScreenMode::C128(80),
    ScreenMode::Atari,
    ScreenMode::AtariXep80,
];

impl Display for ScreenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenMode::DOS(w, h) => write!(f, "{}x{}", w, h),
            ScreenMode::C64 => write!(f, "C64"),
            ScreenMode::C128(col) =>  write!(f, "C128 ({} col)", col),
            ScreenMode::Atari => write!(f, "Atari"),
            ScreenMode::AtariXep80 => write!(f, "Atari XEP80"),
        }
    }
}

impl ScreenMode {
    pub fn parse(str: &str) -> Option<Self>
    {
        match str { 
            "C64" => Some(ScreenMode::C64),
            "C128" | "C128#40" => Some(ScreenMode::C128(40)),
            "C128#80" => Some(ScreenMode::C128(80)),
            "Atari" => Some(ScreenMode::Atari),
            "AtariXep80" => Some(ScreenMode::AtariXep80),
            _ => {
                if let Some(o) = str.find('x') {
                    let x = i32::from_str_radix(&str[0..o], 10);
                    let y = i32::from_str_radix(&str[o + 1..], 10);
                    if x.is_ok() && y.is_ok() {
                        return Some(ScreenMode::DOS(x.unwrap(), y.unwrap()));
                    }
                }
                println!("Unknown screen mode :{}", str);
                None
            }
        }
    }

    pub fn set_mode(&self, font: &mut Option<String>, buffer_view: &mut BufferView)
    {
        let buf = &mut buffer_view.buf;
        match self {
            ScreenMode::DOS(w, h) => {
                buf.set_buffer_width(*w);
                buf.set_buffer_height(*h);
                if *h >= 50 {
                    *font = Some("IBM VGA50".to_string());
                } else {
                    *font = Some("IBM VGA".to_string());
                }
                buffer_view.buffer_parser = Box::new(AvatarParser::new(true));
                buffer_view.petscii = false;
                buf.palette = Palette::new();
            }
            ScreenMode::C64 => {
                buf.set_buffer_width(40);
                buf.set_buffer_height(25);
                *font = Some("C64 PETSCII unshifted".to_string());
                buf.extended_font = Some(BitFont::from_name(&"C64 PETSCII shifted").unwrap());
                buffer_view.buffer_parser = Box::new(PETSCIIParser::new());
                buffer_view.petscii = true;
                buf.palette = Palette { colors: C64_DEFAULT_PALETTE.to_vec() };
            }
            ScreenMode::C128(col) => {
                buf.set_buffer_width(*col);
                buf.set_buffer_height(25);
                *font = Some("C64 PETSCII unshifted".to_string());
                buf.extended_font = Some(BitFont::from_name(&"C64 PETSCII shifted").unwrap());
                buffer_view.buffer_parser = Box::new(PETSCIIParser::new());
                buffer_view.petscii = true;
                buf.palette = Palette { colors: C64_DEFAULT_PALETTE.to_vec() };
            },
            ScreenMode::Atari =>  {
                buf.set_buffer_width(40);
                buf.set_buffer_height(40);
                *font = Some("Atari ATASCII".to_string());
                buffer_view.buffer_parser = Box::new(AnsiParser::new());
                buffer_view.petscii = false;
                buf.palette = Palette::new();
            },
            ScreenMode::AtariXep80 =>  {
                buf.set_buffer_width(40);
                buf.set_buffer_height(30);
                *font = Some("Atari ATASCII".to_string());
                buffer_view.buffer_parser = Box::new(AnsiParser::new());
                buffer_view.petscii = false;
                buf.palette = Palette::new();
            },
        }
        buf.clear();
    }
}

