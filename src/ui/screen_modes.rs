use std::fmt::Display;

use icy_engine::{
    AnsiParser, AtasciiParser, AvatarParser, BitFont, PETSCIIParser, Palette, Size, ViewdataParser,
    ATARI_DEFAULT_PALETTE, C64_DEFAULT_PALETTE, VIEWDATA_PALETTE,
};
use serde_derive::{Deserialize, Serialize};

use super::{main_window_mod::MainWindow, BufferInputMode};

//use super::{BufferInputMode, BufferView};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "name", content = "par")]
pub enum ScreenMode {
    Default,
    Cga(i32, i32),
    Ega(i32, i32),
    Dos(i32, i32),
    Vic,
    Antic,
    Videotex,
}

pub const DEFAULT_MODES: [ScreenMode; 22] = [
    ScreenMode::Dos(80, 25),
    ScreenMode::Dos(80, 28),
    ScreenMode::Dos(80, 30),
    ScreenMode::Dos(80, 43),
    ScreenMode::Dos(80, 50),
    ScreenMode::Dos(80, 60),
    ScreenMode::Dos(132, 37),
    ScreenMode::Dos(132, 52),
    ScreenMode::Dos(132, 25),
    ScreenMode::Dos(132, 28),
    ScreenMode::Dos(132, 30),
    ScreenMode::Dos(132, 34),
    ScreenMode::Dos(132, 43),
    ScreenMode::Dos(132, 50),
    ScreenMode::Dos(132, 60),
    ScreenMode::C64,
    ScreenMode::C128(40),
    ScreenMode::C128(80),
    ScreenMode::Atari,
    ScreenMode::AtariXep80,
    ScreenMode::VT500,
    ScreenMode::ViewData,
];

impl Display for ScreenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenMode::Dos(w, h) => write!(f, "{w}x{h}"),
            ScreenMode::C64 => write!(f, "C64"),
            ScreenMode::C128(col) => write!(f, "C128 ({col} col)"),
            ScreenMode::Atari => write!(f, "Atari"),
            ScreenMode::AtariXep80 => write!(f, "Atari XEP80"),
            ScreenMode::VT500 => write!(f, "VT500"),
            ScreenMode::ViewData => write!(f, "Viewdata"),
            ScreenMode::NotSet => panic!(),
        }
    }
}

impl ScreenMode {
    pub fn get_input_mode(&self) -> BufferInputMode {
        match self {
            ScreenMode::Dos(_, _) => BufferInputMode::CP437,
            ScreenMode::C64 | ScreenMode::C128(_) => BufferInputMode::PETscii,
            ScreenMode::Atari | ScreenMode::AtariXep80 => BufferInputMode::ATAscii,
            ScreenMode::VT500 => BufferInputMode::VT500,
            ScreenMode::ViewData => BufferInputMode::ViewData,
            ScreenMode::NotSet => panic!(),
        }
    }

    pub fn get_window_size(&self) -> Size<u16> {
        match self {
            ScreenMode::Dos(w, h) => {
                Size::new(u16::try_from(*w).unwrap(), u16::try_from(*h).unwrap())
            }
            ScreenMode::C64 => Size::new(40, 25),
            ScreenMode::C128(w) => Size::new(u16::try_from(*w).unwrap(), 25),
            ScreenMode::Atari | ScreenMode::ViewData => Size::new(40, 24),
            ScreenMode::AtariXep80 | ScreenMode::VT500 => Size::new(80, 25),
            ScreenMode::NotSet => panic!(),
        }
    }

    pub fn set_mode(&self, main_window: &mut MainWindow) {
        let buf = &mut main_window.buffer_view.lock().buf;
        buf.set_buffer_size(self.get_window_size());
        match self {
            ScreenMode::Dos(_, h) => {
                buf.font_table.clear();
                buf.font_table.push(
                    BitFont::from_name(if *h >= 50 { "IBM VGA50" } else { "IBM VGA" }).unwrap(),
                );

                main_window.buffer_parser = Box::new(AvatarParser::new(true));
                buf.palette = Palette::new();
            }
            ScreenMode::C64 | ScreenMode::C128(_) => {
                buf.font_table.clear();
                buf.font_table
                    .push(BitFont::from_name("C64 PETSCII unshifted").unwrap());
                buf.font_table
                    .push(BitFont::from_name("C64 PETSCII shifted").unwrap());
                main_window.buffer_parser = Box::<PETSCIIParser>::default();
                buf.palette = Palette {
                    colors: C64_DEFAULT_PALETTE.to_vec(),
                };
            }
            ScreenMode::Atari | ScreenMode::AtariXep80 => {
                buf.font_table.clear();
                buf.font_table
                    .push(BitFont::from_name("Atari ATASCII").unwrap());

                main_window.buffer_parser = Box::<AtasciiParser>::default();
                buf.palette = Palette {
                    colors: ATARI_DEFAULT_PALETTE.to_vec(),
                };
            }
            ScreenMode::VT500 => {
                buf.font_table.clear();
                buf.font_table.push(BitFont::from_name("IBM VGA").unwrap());
                main_window.buffer_parser = Box::new(AnsiParser::new());
                buf.palette = Palette::new();
            }
            ScreenMode::ViewData => {
                buf.font_table.clear();
                buf.font_table.push(BitFont::from_name("Viewdata").unwrap());
                main_window.buffer_parser = Box::new(ViewdataParser::new());
                buf.palette = Palette {
                    colors: VIEWDATA_PALETTE.to_vec(),
                };
            }
            ScreenMode::NotSet => panic!(),
        }
        buf.clear();
    }
}
