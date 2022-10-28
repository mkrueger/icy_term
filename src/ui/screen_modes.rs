use std::fmt::Display;

use crate::{model::{Buffer, DosChar, BitFont}};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScreenMode {
    DOS(u16, u16),
    C64,
    C128(u16),
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
                    let x = u16::from_str_radix(&str[0..o], 10);
                    let y = u16::from_str_radix(&str[o + 1..], 10);
                    if x.is_ok() && y.is_ok() {
                        return Some(ScreenMode::DOS(x.unwrap(), y.unwrap()));
                    }
                }
                println!("Unknown screen mode :{}", str);
                None
            }
        }
    }


    pub fn set_mode(&self, font: &mut Option<String>, buf: &mut Buffer)
    {
        match self {
            ScreenMode::DOS(w, h) => {
                buf.width = *w;
                buf.height = *h;
                if *h >= 50 {
                    *font = Some("IBM VGA50".to_string());
                } else {
                    *font = Some("IBM VGA".to_string());
                }
                buf.petscii = false;
                buf.palette = crate::model::Palette::new();
            }
            ScreenMode::C64 => {
                buf.width = 40;
                buf.height = 40;
                *font = Some("C64 PETSCII unshifted".to_string());
                buf.extended_font = Some(BitFont::from_name(&"C64 PETSCII shifted").unwrap());
                buf.petscii = true;
                buf.palette = crate::model::Palette { colors: crate::model::C64_DEFAULT_PALETTE.to_vec() };
            }
            ScreenMode::C128(col) => {
                buf.width = 40;
                buf.height = *col;
                *font = Some("C64 PETSCII unshifted".to_string());
                buf.extended_font = Some(BitFont::from_name(&"C64 PETSCII shifted").unwrap());
                buf.petscii = true;
                buf.palette = crate::model::Palette { colors: crate::model::C64_DEFAULT_PALETTE.to_vec() };
            },
            ScreenMode::Atari =>  {
                buf.width = 40;
                buf.height = 40;
                *font = Some("Atari ATASCII".to_string());
                buf.petscii = false;
                buf.palette = crate::model::Palette::new();
            },
            ScreenMode::AtariXep80 =>  {
                buf.width = 40;
                buf.height = 30;
                *font = Some("Atari ATASCII".to_string());
                buf.petscii = false;
                buf.palette = crate::model::Palette::new();
            },
        }

        for y in 0..buf.height {
            for x in 0..buf.width {
                let p = crate::model::Position::from(x as i32, y as i32);
                if buf.get_char(p).is_none() {
                    buf.set_char(p, Some(DosChar::default()));
                }
            }
        }
    }
}

