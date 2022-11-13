mod ui;
use iced::{Settings, Application};
use ui::*;

mod com;
mod address;

mod iemsi;
mod protocol;
mod auto_login;
mod auto_file_transfer;

const VERSION: &str = env!("CARGO_PKG_VERSION");
pub fn main() -> iced::Result {
    MainWindow::run(Settings {
        window: iced::window::Settings {
            size: (880, 590),
            transparent: true,
            ..Default::default()
        },
        antialiasing: true,
        
        ..Settings::default()
    })
}