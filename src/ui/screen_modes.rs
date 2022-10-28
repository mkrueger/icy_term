use std::fmt::Display;

use crate::model::Buffer;


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
    pub fn set_mode(&self, buf: &mut Buffer)
    {
        match self {
            ScreenMode::DOS(w, h) => {
                buf.width = *w;
                buf.height = *h;
            }
            ScreenMode::C64 => {
                buf.width = 40;
                buf.height = 40;
            }
            ScreenMode::C128(col) => {
                buf.width = 40;
                buf.height = *col;
            },
            ScreenMode::Atari =>  {
                buf.width = 40;
                buf.height = 40;
            },
            ScreenMode::AtariXep80 =>  {
                buf.width = 40;
                buf.height = 30;
            },
        }
    }
}

