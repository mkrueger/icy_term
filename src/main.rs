mod ui;
use eframe::egui;
use ui::{*};

mod address;
mod com;

mod auto_file_transfer;
mod auto_login;
mod iemsi;
mod protocol;
mod sound;
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 1000.0)),
        multisampling: 8,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        format!("iCY TERM {} - Offline", VERSION).as_str(),
        options,
        Box::new(|cc| Box::new(MainWindow::new(cc))),
    );
}