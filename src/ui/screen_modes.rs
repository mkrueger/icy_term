use std::fmt::Display;

use icy_engine::{
    AtasciiParser, AvatarParser, BitFont, PETSCIIParser, Palette, Size, ViewdataParser,
    ATARI_DEFAULT_PALETTE, C64_DEFAULT_PALETTE, VIEWDATA_PALETTE,
};

use super::{main_window_mod::MainWindow, BufferInputMode};

//use super::{BufferInputMode, BufferView};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScreenMode {
    Default,
    Cga(i32, i32),
    Ega(i32, i32),
    Vga(i32, i32),
    Vic,
    Antic,
    Videotex,
}

pub const DEFAULT_MODES: [ScreenMode; 18] = [
    ScreenMode::Vga(80, 25),
    ScreenMode::Vga(80, 28),
    ScreenMode::Vga(80, 30),
    ScreenMode::Vga(80, 43),
    ScreenMode::Vga(80, 50),
    ScreenMode::Vga(80, 60),
    ScreenMode::Vga(132, 37),
    ScreenMode::Vga(132, 52),
    ScreenMode::Vga(132, 25),
    ScreenMode::Vga(132, 28),
    ScreenMode::Vga(132, 30),
    ScreenMode::Vga(132, 34),
    ScreenMode::Vga(132, 43),
    ScreenMode::Vga(132, 50),
    ScreenMode::Vga(132, 60),
    ScreenMode::Vic,
    ScreenMode::Antic,
    ScreenMode::Videotex,
];

impl Display for ScreenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenMode::Vga(w, h) => write!(f, "VGA {w}x{h}"),
            ScreenMode::Ega(w, h) => write!(f, "EGA {w}x{h}"),
            ScreenMode::Cga(w, h) => write!(f, "CGA {w}x{h}"),
            ScreenMode::Vic => write!(f, "C64"),
            ScreenMode::Antic => write!(f, "Atari"),
            ScreenMode::Videotex => write!(f, "Viewdata"),
            ScreenMode::Default => write!(f, "Default"),
        }
    }
}

impl ScreenMode {
    pub fn get_input_mode(&self) -> BufferInputMode {
        match self {
            ScreenMode::Cga(_, _) | ScreenMode::Ega(_, _) | ScreenMode::Vga(_, _) => {
                BufferInputMode::CP437
            }
            ScreenMode::Vic => BufferInputMode::PETscii,
            ScreenMode::Antic => BufferInputMode::ATAscii,
            ScreenMode::Videotex => BufferInputMode::ViewData,
            ScreenMode::Default => BufferInputMode::CP437,
        }
    }

    pub fn get_window_size(&self) -> Size<u16> {
        match self {
            ScreenMode::Cga(w, h) | ScreenMode::Ega(w, h) | ScreenMode::Vga(w, h) => {
                Size::new(u16::try_from(*w).unwrap(), u16::try_from(*h).unwrap())
            }
            ScreenMode::Vic => Size::new(40, 25),
            ScreenMode::Antic | ScreenMode::Videotex => Size::new(40, 24),
            ScreenMode::Default => Size::new(80, 25),
        }
    }

    pub fn set_mode(&self, main_window: &mut MainWindow) {
        let buf = &mut main_window.buffer_view.lock().buf;
        buf.set_buffer_size(self.get_window_size());
        match self {
            ScreenMode::Default => {
                buf.font_table.clear();
                buf.font_table.push(BitFont::from_name("IBM VGA").unwrap());
                main_window.buffer_parser = Box::new(AvatarParser::new(true));
                buf.palette = Palette::new();
            }
            ScreenMode::Cga(_, h) | ScreenMode::Ega(_, h) | ScreenMode::Vga(_, h) => {
                buf.font_table.clear();
                buf.font_table.push(
                    BitFont::from_name(if *h >= 50 { "IBM VGA50" } else { "IBM VGA" }).unwrap(),
                );

                main_window.buffer_parser = Box::new(AvatarParser::new(true));
                buf.palette = Palette::new();
            }

            ScreenMode::Vic => {
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
            ScreenMode::Antic => {
                buf.font_table.clear();
                buf.font_table
                    .push(BitFont::from_name("Atari ATASCII").unwrap());

                main_window.buffer_parser = Box::<AtasciiParser>::default();
                buf.palette = Palette {
                    colors: ATARI_DEFAULT_PALETTE.to_vec(),
                };
            }
            ScreenMode::Videotex => {
                buf.font_table.clear();
                buf.font_table.push(BitFont::from_name("Viewdata").unwrap());
                main_window.buffer_parser = Box::new(ViewdataParser::new());
                buf.palette = Palette {
                    colors: VIEWDATA_PALETTE.to_vec(),
                };
            }
        }
        buf.clear();
    }
}
