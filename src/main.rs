#![warn(clippy::all, clippy::pedantic)]
mod ui;
use std::error::Error;

use eframe::egui;
use lazy_static::*;
use ui::*;
pub type TerminalResult<T> = Result<T, Box<dyn Error>>;
use i18n_embed::{
    fluent::{fluent_language_loader, FluentLanguageLoader},
    DesktopLanguageRequester,
};
use rust_embed::RustEmbed;

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

#[derive(RustEmbed)]
#[folder = "i18n"] // path to the compiled localization resources
struct Localizations;

use once_cell::sync::Lazy;
static LANGUAGE_LOADER: Lazy<FluentLanguageLoader> = Lazy::new(|| {
    let loader = fluent_language_loader!();
    let requested_languages = DesktopLanguageRequester::requested_languages();
    let _result = i18n_embed::select(&loader, &Localizations, &requested_languages);
    loader
});

#[tokio::main]
async fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1284. + 8., 839.)),
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
