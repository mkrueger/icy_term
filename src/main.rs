mod ui;
use std::error::Error;

use eframe::egui;
use lazy_static::*;
use ui::*;

mod address;
mod com;

mod auto_file_transfer;
mod auto_login;
mod iemsi;
mod protocol;
mod sound;
const VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    static ref DEFAULT_TITLE: String = format!("iCY TERM {}", crate::VERSION);
}

pub type TerminalResult<T> = Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1280., 841.)),
        multisampling: 0,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        &DEFAULT_TITLE,
        options,
        Box::new(|cc| Box::new(MainWindow::new(cc))),
    );
}
