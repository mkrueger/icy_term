use std::fmt::Display;

use egui::Color32;
use icy_engine::{
    BitFont, Palette, Size, ATARI_DEFAULT_PALETTE, C64_DEFAULT_PALETTE, VIEWDATA_PALETTE,
};
use icy_engine_egui::{BufferInputMode, FontExtension};

use crate::ui::MainWindow;

//use super::{BufferInputMode, BufferView};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScreenMode {
    Default,
    // Cga(i32, i32),
    // Ega(i32, i32),
    Vga(i32, i32),
    Vic,
    Antic,
    Videotex,
}

impl Default for ScreenMode {
    fn default() -> Self {
        ScreenMode::Vga(80, 25)
    }
}

pub const DEFAULT_MODES: [ScreenMode; 8] = [
    ScreenMode::Vga(80, 25),
    ScreenMode::Vga(80, 50),
    ScreenMode::Default,
    ScreenMode::Vic,
    ScreenMode::Default,
    ScreenMode::Antic,
    ScreenMode::Default,
    ScreenMode::Videotex,
];

impl Display for ScreenMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenMode::Vga(w, h) => write!(f, "VGA {w}x{h}"),
            // ScreenMode::Ega(w, h) => write!(f, "EGA {w}x{h}"),
            // ScreenMode::Cga(w, h) => write!(f, "CGA {w}x{h}"),
            ScreenMode::Vic => write!(f, "VIC-II"),
            ScreenMode::Antic => write!(f, "ANTIC"),
            ScreenMode::Videotex => write!(f, "VIDEOTEX"),
            ScreenMode::Default => write!(f, "Default"),
        }
    }
}

impl ScreenMode {
    pub fn get_font_extension(&self) -> FontExtension {
        FontExtension::Off
        /*
        match self {
            ScreenMode::Default | ScreenMode::Vga(_, _) => FontExtension::LineGraphicsEnable,
            _ => FontExtension::Off,
        }*/
    }

    pub fn get_input_mode(&self) -> BufferInputMode {
        match self {
            //ScreenMode::Cga(_, _) | ScreenMode::Ega(_, _) |
            ScreenMode::Default | ScreenMode::Vga(_, _) => BufferInputMode::CP437,
            ScreenMode::Vic => BufferInputMode::PETscii,
            ScreenMode::Antic => BufferInputMode::ATAscii,
            ScreenMode::Videotex => BufferInputMode::ViewData,
        }
    }

    pub fn get_window_size(&self) -> Size<u16> {
        match self {
            // ScreenMode::Cga(w, h) | ScreenMode::Ega(w, h) |
            ScreenMode::Vga(w, h) => {
                Size::new(u16::try_from(*w).unwrap(), u16::try_from(*h).unwrap())
            }
            ScreenMode::Vic => Size::new(40, 25),
            ScreenMode::Antic | ScreenMode::Videotex => Size::new(40, 24),
            ScreenMode::Default => Size::new(80, 25),
        }
    }

    pub fn set_mode(&self, main_window: &MainWindow) {
        let buf = &mut main_window.buffer_view.lock().buf;
        buf.set_buffer_size(self.get_window_size());
        match self {
            ScreenMode::Default => {
                buf.clear_font_table();
                buf.set_font(0, BitFont::from_name("IBM VGA").unwrap());
                buf.palette = Palette::new();
            }
            // ScreenMode::Cga(_, h) | ScreenMode::Ega(_, h) |
            ScreenMode::Vga(_, h) => {
                buf.clear_font_table();
                buf.set_font(
                    0,
                    BitFont::from_name(if *h >= 50 { "IBM VGA50" } else { "IBM VGA" }).unwrap(),
                );
                buf.palette = Palette::new();
            }

            ScreenMode::Vic => {
                buf.clear_font_table();
                buf.set_font(0, BitFont::from_name("C64 PETSCII unshifted").unwrap());
                buf.set_font(1, BitFont::from_name("C64 PETSCII shifted").unwrap());

                buf.palette = Palette {
                    colors: C64_DEFAULT_PALETTE.to_vec(),
                };
            }
            ScreenMode::Antic => {
                buf.clear_font_table();
                buf.set_font(0, BitFont::from_name("Atari ATASCII").unwrap());
                buf.palette = Palette {
                    colors: ATARI_DEFAULT_PALETTE.to_vec(),
                };
            }
            ScreenMode::Videotex => {
                buf.clear_font_table();
                buf.set_font(0, BitFont::from_name("Viewdata").unwrap());
                buf.palette = Palette {
                    colors: VIEWDATA_PALETTE.to_vec(),
                };
            }
        }
        buf.clear();
    }

    #[allow(clippy::match_same_arms)]
    pub(crate) fn get_selection_fg(&self) -> egui::Color32 {
        match self {
            ScreenMode::Default | ScreenMode::Vga(_, _) => Color32::from_rgb(0xAA, 0x00, 0xAA),
            ScreenMode::Vic => Color32::from_rgb(0x37, 0x39, 0xC4),
            ScreenMode::Antic => Color32::from_rgb(0x09, 0x51, 0x83),
            ScreenMode::Videotex => Color32::from_rgb(0, 0, 0),
        }
    }

    #[allow(clippy::match_same_arms)]
    pub(crate) fn get_selection_bg(&self) -> egui::Color32 {
        match self {
            ScreenMode::Default | ScreenMode::Vga(_, _) => Color32::from_rgb(0xAA, 0xAA, 0xAA),
            ScreenMode::Vic => Color32::from_rgb(0xB0, 0x3F, 0xB6),
            ScreenMode::Antic => Color32::from_rgb(0xFF, 0xFF, 0xFF),
            ScreenMode::Videotex => Color32::from_rgb(0xFF, 0xFF, 0xFF),
        }
    }
}
