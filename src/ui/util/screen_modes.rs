use std::fmt::Display;

use icy_engine::{
    BitFont, Color, Palette, Size, ATARI, ATARI_DEFAULT_PALETTE, C64_DEFAULT_PALETTE, C64_LOWER,
    C64_UPPER, CP437, VIEWDATA, VIEWDATA_PALETTE,
};
use icy_engine_egui::BufferInputMode;

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

pub const DEFAULT_MODES: [ScreenMode; 10] = [
    ScreenMode::Vga(80, 25),
    ScreenMode::Vga(80, 50),
    ScreenMode::Vga(132, 37),
    ScreenMode::Vga(132, 52),
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
    pub fn get_input_mode(&self) -> BufferInputMode {
        match self {
            //ScreenMode::Cga(_, _) | ScreenMode::Ega(_, _) |
            ScreenMode::Default | ScreenMode::Vga(_, _) => BufferInputMode::CP437,
            ScreenMode::Vic => BufferInputMode::PETscii,
            ScreenMode::Antic => BufferInputMode::ATAscii,
            ScreenMode::Videotex => BufferInputMode::ViewData,
        }
    }

    pub fn get_window_size(&self) -> Size {
        match self {
            // ScreenMode::Cga(w, h) | ScreenMode::Ega(w, h) |
            ScreenMode::Vga(w, h) => Size::new(*w, *h),
            ScreenMode::Vic => Size::new(40, 25),
            ScreenMode::Antic | ScreenMode::Videotex => Size::new(40, 24),
            ScreenMode::Default => Size::new(80, 25),
        }
    }

    pub fn set_mode(&self, main_window: &MainWindow) {
        main_window
            .buffer_view
            .lock()
            .get_buffer_mut()
            .set_size(self.get_window_size());
        match self {
            // ScreenMode::Cga(_, h) | ScreenMode::Ega(_, h) |
            ScreenMode::Vga(_, _) | ScreenMode::Default => {
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .clear_font_table();
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .set_font(0, BitFont::from_bytes("", CP437).unwrap());
                main_window.buffer_view.lock().get_buffer_mut().palette = Palette::dos_default();
            }

            ScreenMode::Vic => {
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .clear_font_table();
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .set_font(0, BitFont::from_bytes("", C64_LOWER).unwrap());
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .set_font(1, BitFont::from_bytes("", C64_UPPER).unwrap());

                main_window.buffer_view.lock().get_buffer_mut().palette =
                    Palette::from_colors(C64_DEFAULT_PALETTE.to_vec());
            }
            ScreenMode::Antic => {
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .clear_font_table();
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .set_font(0, BitFont::from_bytes("", ATARI).unwrap());
                main_window.buffer_view.lock().get_buffer_mut().palette =
                    Palette::from_colors(ATARI_DEFAULT_PALETTE.to_vec());
            }
            ScreenMode::Videotex => {
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .clear_font_table();
                main_window
                    .buffer_view
                    .lock()
                    .get_buffer_mut()
                    .set_font(0, BitFont::from_bytes("", VIEWDATA).unwrap());
                main_window.buffer_view.lock().get_buffer_mut().palette =
                    Palette::from_colors(VIEWDATA_PALETTE.to_vec());
            }
        }
        main_window.buffer_view.lock().get_buffer_mut().layers[0].clear();
        main_window
            .buffer_view
            .lock()
            .get_buffer_mut()
            .stop_sixel_threads();
    }

    #[allow(clippy::match_same_arms)]
    pub(crate) fn get_selection_fg(&self) -> Color {
        match self {
            ScreenMode::Default | ScreenMode::Vga(_, _) => Color::new(0xAA, 0x00, 0xAA),
            ScreenMode::Vic => Color::new(0x37, 0x39, 0xC4),
            ScreenMode::Antic => Color::new(0x09, 0x51, 0x83),
            ScreenMode::Videotex => Color::new(0, 0, 0),
        }
    }

    #[allow(clippy::match_same_arms)]
    pub(crate) fn get_selection_bg(&self) -> Color {
        match self {
            ScreenMode::Default | ScreenMode::Vga(_, _) => Color::new(0xAA, 0xAA, 0xAA),
            ScreenMode::Vic => Color::new(0xB0, 0x3F, 0xB6),
            ScreenMode::Antic => Color::new(0xFF, 0xFF, 0xFF),
            ScreenMode::Videotex => Color::new(0xFF, 0xFF, 0xFF),
        }
    }
}
