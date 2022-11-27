#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(unsafe_code)]

use std::{sync::Arc, env};
use egui::mutex::Mutex;
use icy_engine::{DEFAULT_FONT_NAME, BufferParser, AvatarParser};
use std::time::{Duration, SystemTime};

use eframe::{egui::{self}, epaint::Vec2};

use crate::address::{Address};
use crate::auto_file_transfer::AutoFileTransfer;
use crate::auto_login::AutoLogin;
use crate::com::{Com};
use crate::protocol::{Protocol, TransferState};

use super::BufferView;

#[derive(PartialEq, Eq)]
pub enum MainWindowMode {
    ShowTerminal,
    ShowPhonebook,
    SelectProtocol(bool),
    FileTransfer(bool),
    AskDeleteEntry
}

struct Options {
    connect_timeout: Duration,
}

impl Options {
    pub fn new() -> Self {
        Options {
            connect_timeout: Duration::from_secs(10),
        }
    }
}

pub struct MainWindow {
    com: Option<Box<dyn Com>>,
    buffer_view: Arc<Mutex<BufferView>>,
    buffer_parser: Box<dyn BufferParser>,
    
    trigger: bool,
    pub mode: MainWindowMode,
    pub addresses: Vec<Address>,
    pub handled_char: bool,
    cur_addr: usize,
    options: Options,
    connection_time: SystemTime,
    font: Option<String>,
   // screen_mode: Option<ScreenMode>,
    auto_login: AutoLogin,
    auto_file_transfer: AutoFileTransfer,
    // protocols
    current_protocol: Option<(Box<dyn Protocol>, TransferState)>,
    is_alt_pressed: bool,
}

impl MainWindow {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let gl = cc
            .gl
            .as_ref()
            .expect("You need to run eframe with the glow backend");
        
        let view  = BufferView::new(gl);

        let mut view = MainWindow {
            buffer_view: Arc::new(Mutex::new(view)),
            //address_list: HoverList::new(),
            com: None,
            trigger: true,
            mode: MainWindowMode::ShowPhonebook,
            addresses: Vec::new(), // start_read_book(),
            cur_addr: 0,
            connection_time: SystemTime::now(),
            options: Options::new(),
            auto_login: AutoLogin::new(String::new()),
            auto_file_transfer: AutoFileTransfer::new(),
            font: Some(DEFAULT_FONT_NAME.to_string()),
            //screen_mode: None,
            current_protocol: None,
            handled_char: false,
            is_alt_pressed: false,
            buffer_parser: Box::new(AvatarParser::new(true)),
        };
        let args: Vec<String> = env::args().collect();
        if let Some(arg) = args.get(1) {
            view.addresses[0].address = arg.clone();
     //       let cmd = view.call_bbs(0);
        }
        //view.address_list.selected_item = 1;
        // view.set_screen_mode(&ScreenMode::Viewdata);
        //view.update_address_list();

        view
    }

    pub fn print_char(
        &mut self,
        com: Option<&mut dyn Com>,
        c: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        /* 
        match c  {
            b'\\' => print!("\\\\"),
            b'\n' => print!("\\n"),
            b'\r' => print!("\\r"),
            b'\"' => print!("\\\""),
            _ => {
                if c < b' ' || c == b'\x7F' {
                    print!("\\x{:02X}", c as u8);
                } else if c > b'\x7F' {
                    print!("\\u{{{:02X}}}", c as u8);
                } else {
                    print!("{}", char::from_u32(c as u32).unwrap());
                }
            }
        }*/
/* 
        let result = self
            .buffer_parser
            .print_char(&mut self.buf, &mut self.caret, unsafe {
                char::from_u32_unchecked(c as u32)
            })?;
        match result {
            icy_engine::CallbackAction::None => {},
            icy_engine::CallbackAction::SendString(result) => {
                if let Some(com) = com {
                    com.write(result.as_bytes())?;
                }
            },
            icy_engine::CallbackAction::PlayMusic(music) => play_music(music)
        }
        //if !self.update_sixels() {
            self.redraw_view();
        //}*/
        Ok(())
    }
}


impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.custom_painting(ui);
            });
        });
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            self.buffer_view.lock().destroy(gl);
        }
    }
}

impl MainWindow {
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let buffer_view = self.buffer_view.clone();

            let h = buffer_view.lock().buf.get_real_buffer_height() as f32;

            let (rect, response) = ui.allocate_at_least(Vec2::new(size.x, h * 16.), egui::Sense::drag());

            
            let used_rect = ui.ctx().used_rect();
            let callback = egui::PaintCallback {
                rect: rect,
                callback: std::sync::Arc::new(egui_glow::CallbackFn::new(move |_info, painter| {
                    buffer_view.lock().paint(painter.gl(), rect, size);
                })),
            };
            ui.painter().add(callback);
        });
    }
}
