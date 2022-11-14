mod ui;
use iced::{Application, Settings};
use ui::*;

mod address;
mod com;

mod auto_file_transfer;
mod auto_login;
mod iemsi;
mod protocol;

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
