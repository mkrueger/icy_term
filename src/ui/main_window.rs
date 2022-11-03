use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::net::{ToSocketAddrs};
use iced::keyboard::{KeyCode};
use iced::mouse::ScrollDelta;
use iced::widget::{Canvas, column, row, button, text, pick_list};
use iced::{executor, subscription, Event, keyboard, mouse};
use iced::{
    Application, Command, Element, Length, 
    Subscription, Theme,
};
use iced::widget::{
     text_input, vertical_rule
};
use iced::{Alignment};
use icy_engine::{SUPPORTED_FONTS, DEFAULT_FONT_NAME, BitFont};
use rfd::FileDialog;

use crate::auto_login::AutoLogin;
use crate::{VERSION};
use crate::address::{Address, start_read_book, READ_ADDRESSES, store_phone_book};
use crate::com::{Com, TelnetCom};
use crate::protocol::{ Zmodem, XYmodem, Protocol, ProtocolType, FileDescriptor, XYModemVariant};

use super::{BufferView, Message};
use super::screen_modes::{DEFAULT_MODES, ScreenMode};

enum MainWindowMode {
    Default,
    ShowPhonebook,
    SelectProtocol(bool),
    FileTransfer(ProtocolType, bool),
    EditBBS(usize)
}

struct Options {
    connect_timeout: Duration
}

impl Options {
    pub fn new() -> Self {
        Options {
            connect_timeout: Duration::from_secs(10)
        }
    }
}

pub struct MainWindow<T: Com> {
    pub buffer_view: BufferView,
    com: Option<T>,
    trigger: bool,
    mode: MainWindowMode,
    pub addresses: Vec<Address>,
    edit_bbs: Address,
    cur_addr: usize,
    log_file: Vec<String>,
    options: Options,
    connection_time: SystemTime,
    font: Option<String>,
    screen_mode: Option<ScreenMode>,
    auto_login: AutoLogin,
    // protocols
    xymodem: XYmodem,
    zmodem: Zmodem
}



static KEY_MAP: &[(KeyCode, &[u8])] = &[
    (KeyCode::Home, "\x1b[1~".as_bytes()),
    (KeyCode::Insert, "\x1b[2~".as_bytes()),
    (KeyCode::Backspace, &[8]),
    (KeyCode::Delete, "\x1b[3~".as_bytes()),
    (KeyCode::End, "\x1b[4~".as_bytes()),
    (KeyCode::PageUp, "\x1b[5~".as_bytes()),
    (KeyCode::PageDown, "\x1b[6~".as_bytes()),
    (KeyCode::F1, "\x1b[11~".as_bytes()),
    (KeyCode::F2, "\x1b[12~".as_bytes()),
    (KeyCode::F3, "\x1b[13~".as_bytes()),
    (KeyCode::F4, "\x1b[14~".as_bytes()),
    (KeyCode::F5, "\x1b[15~".as_bytes()),
    (KeyCode::F6, "\x1b[17~".as_bytes()),
    (KeyCode::F7, "\x1b[18~".as_bytes()),
    (KeyCode::F8, "\x1b[19~".as_bytes()),
    (KeyCode::F9, "\x1b[20~".as_bytes()),
    (KeyCode::F10, "\x1b[21~".as_bytes()),
    (KeyCode::F11, "\x1b[23~".as_bytes()),
    (KeyCode::F12, "\x1b[24~".as_bytes()),
    (KeyCode::Up, "\x1b[A".as_bytes()),
    (KeyCode::Down, "\x1b[B".as_bytes()),
    (KeyCode::Right, "\x1b[C".as_bytes()),
    (KeyCode::Left, "\x1b[D".as_bytes())
];

static C64_KEY_MAP: &[(KeyCode, &[u8])] = &[
    (KeyCode::Home, &[0x13]),
    (KeyCode::Insert, &[0x94]),
    (KeyCode::Backspace, &[0x14]),
    (KeyCode::Delete, &[0x14]),
    (KeyCode::F1, &[0x85]),
    (KeyCode::F2, &[0x86]),
    (KeyCode::F3, &[0x87]),
    (KeyCode::F4, &[0x88]),
    (KeyCode::F5, &[0x89]),
    (KeyCode::F6, &[0x8A]),
    (KeyCode::F7, &[0x8B]),
    (KeyCode::F8, &[0x8C]),

    (KeyCode::Up, &[0x91]),
    (KeyCode::Down, &[0x11]),
    (KeyCode::Right, &[0x1D]),
    (KeyCode::Left, &[0x9D])
];

impl MainWindow<TelnetCom> 
{
    pub fn update_state(&mut self) -> io::Result<()>
    {
        match &mut self.com {
            None => Ok(()),
            Some(com) => {
                if let Some(adr) = self.addresses.get(self.cur_addr) {
                    if let Err(err) = self.auto_login.run_autologin(com, adr) {
                        eprintln!("{}", err);
                        self.log_file.push(format!("{}", err));
                    }
                }
                let mut do_update = false;
                while com.is_data_available()? {
                    let ch = com.read_char_nonblocking()?;
                    if let Some(adr) = self.addresses.get(self.cur_addr) {
                        if let Err(err) = self.auto_login.try_login(com, adr, ch) {
                            eprintln!("{}", err);
                            self.log_file.push(format!("{}", err));
                        }
                    }
                    self.buffer_view.print_char(Some(com), ch)?;
                    do_update = true;
                }
                if do_update {
                    self.buffer_view.cache.clear();
                }
                Ok(())
            }
        }
    }

    pub fn get_screen_mode(&self) -> ScreenMode
    {
        if let Some(mode) = self.screen_mode {
            return mode;
        }

        return ScreenMode::DOS(80, 25);
    }

    pub fn get_font_name(&self) -> String
    {
        if let Some(font) = &self.font {
            return font.clone();
        }

        return DEFAULT_FONT_NAME.to_string();
    }
    
    pub fn print_log(&mut self, str: String)
    {
        self.log_file.push(str);
    }

    pub fn print_result<T>(&mut self, result: &io::Result<T>)
    {
        if let Err(error) = result {
            eprintln!("{}", error);
            self.log_file.push(format!("{}", error));
        }
    }

    pub fn set_font(&mut self, font: &String)
    {
        if font != &self.get_font_name() { 
            self.font = Some(font.clone());
            self.buffer_view.buf.font = BitFont::from_name(&self.get_font_name()).unwrap();
            self.buffer_view.cache.clear();
        }
    }

    pub fn set_screen_mode(&mut self, mode: &ScreenMode)
    {
        self.screen_mode = Some(*mode);
        self.get_screen_mode().set_mode(&mut self.font, &mut self.buffer_view);
        self.buffer_view.buf.font = BitFont::from_name(&self.get_font_name()).unwrap();
        self.buffer_view.cache.clear();
    }

    pub fn output_char(&mut self, ch: char) 
    {
        let translated_char = self.buffer_view.buffer_parser.from_unicode(ch);

        if let Some(com) = &mut self.com {
            let state = com.write(&[translated_char]);
            if let Err(err) = state {
                eprintln!("{}", err);
                self.print_log(format!("Error: {:?}", err));
                self.com = None;
            }
        } else {
            let r = self.buffer_view.print_char::<TelnetCom>(Option::<&mut TelnetCom>::None, translated_char);
            self.print_result(&r);
            self.buffer_view.cache.clear();
        }
    }
}

impl Application for MainWindow<TelnetCom> {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }

    fn title(&self) -> String {
        format!("iCY TERM {}", VERSION)
    }

    fn new(_flags: ()) ->  (Self, Command<Message>) {
       let mut view =  MainWindow {
            buffer_view: BufferView::new(),
            com:None,
            trigger: true,
            mode: MainWindowMode::Default,
            addresses: start_read_book(),
            edit_bbs: Address::new(),
            cur_addr: 0,
            connection_time: SystemTime::now(),
            log_file: Vec::new(),
            options: Options::new(),
            auto_login: AutoLogin::new(String::new()),
            xymodem: XYmodem::new(XYModemVariant::XModem),
            zmodem: Zmodem::new(1024),
            font: Some(DEFAULT_FONT_NAME.to_string()),
            screen_mode: None,
        };
 /* 
        let txt = b"\x1B[0m\xDB\x1B[1m\xDB\x1B[0m\xDB\x1B[40m\x1B[K\r\r\n     \xDB\xDB\xDB\xDB    \x1B[1m\xDB\xDB \x1B[0m\xDB\xDB\xDB\xDF     \xDB\xDB\x1B[1m\xDB\xDB \xDC  \xDB   \x1B[31m\xDC\xDC\xDC\xDC\xDC\xDC   \xDC\xDC\x1B[0;31m\xDB\xDB\xDB  \x1B[1;30m\xDF\x1B[0m\xDB\x1B[1;30m\xDF \x1B[31m\xDC\xDB\x1B[0;31m\xDC\x1B[40m\x1B[K\r\r\n     \x1B[0m\xDB\xDB\xDB\xDB    \x1B[1m\xDB\xDB \x1B[0m\xDB\xDB\x1B[1;30m\xDB\xDB \xDB   \x1B[0m\xDB\xDB\x1B[1m\xDB\x1B[30m\xDB \x1B[37m\xDB\xDB\xDB   \x1B[31m\xDB\xDB\xDB \xDB \x1B[0;31m\xDB\x1B[1m\xDB \xDB\xDB\xDB  \x1B[0;31m\xDB  \x1B[1m\xDC\x1B[0;31m\xDB\xDC \x1B[1m\xDB\x1B[0;31m\xDB\xDB       \x1B[0;36mVERSiON\x1B[40m\x1B[K\r\r\n     \x1B[0m\xDB\xDB\xDB\xDB  \x1B[1;30m\xDB \xDB\xDB \x1B[0m\xDB\xDB\x1B[1;30m\xDB\xDC     \x1B[0m\xDB\xDB\xDB\x1B[1;30m\xDB \x1B[37m\xDF  \x1B[30m\xDB  \x1B[31m\xDB\xDB\xDB   \x1B[0;31m\xDB\xDB \x1B[1m\xDB\xDB\xDB  \x1B[0;31m\xDB  \x1B[1m\xDB\x1B[0;31m\xDB\xDB \x1B[1m\xDB\x1B[0;31m\xDB\xDF\x1B[1m\xDC\xDC      \x1B[36m\xFA\xFA\xFA\xFA\xFA\x1B[40m\x1B[K\r\r\n     \x1B[0m\xDF\x1B[1;30m\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB  \x1B[0m\xDF\x1B[1;30m\xDB\xDB\xDB\xDB\xDB\xDB\xDB \x1B[0m\xDF\xDB\x1B[1;30m\xDB\xDB\xDB\xDB\xDB\xDF   \x1B[31m\xDF\xDB\xDF   \x1B[0;31m\xDF\xDB \x1B[1m\xDF\xDB\xDB\xDB\xDB\x1B[0;31m\xDF\xDB\xDC\x1B[1m\xDF\xDB\x1B[0;31m\xDF \x1B[1m\xDF\xDB\xDB\xDB\xDB\xDB\x1B[40m\x1B[K\r\r\n                                                            \xDF\xDF\x1B[40m\x1B[K\r\r\n\x1B[40m\x1B[K\r\r\n\x1B[40m\x1B[K\r\r\n                        \x1B[0;31mDeSiGN bY mULTiDASHER 2007\x1B[40m\x1B[K\r\r\n  \x1B[1;37m\xC0\x1B[0m\xC4\xC4\xC4\x1B[1;30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\x1B[1m\xD9\x1B[40m\x1B[K\r\r\n   \x1B[0;33mLETZTE NACHRiCHT VON :                     \x1B[1mAM \x1B[0;33m:\x1B[40m\x1B[K\r\r\n\x1B[40m\x1B[K\r\r\n\x1B[40m\x1B[K\r\r\n\x1B[40m\x1B[K\r\r\n\x1B[40m\x1B[K\r\r\n   \x1B[0;32mT\x1B[1mE\x1B[0;32mX\x1B[1mT \x1B[0;32mE\x1B[1mi\x1B[0;32mN\x1B[1mG\x1B[0;32mA\x1B[1mB\x1B[0;32mE :\x1B[40m\x1B[K\r\r\n\x1B[0m\xDA\xC4\xC4\x1B[1;30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xBF\x1B[s\r\r\n\xB3\x1B[40m\x1B[K\r\r\n\x1B[1m\xB3\x1B[40m\x1B[K\r\r\n\xC0\x1B[40m\x1B[K\x1B[0m\r\r\n\x1B[7;69H\x1B[1;30m\x1B[1m 1.1o \x1B[m\r\n\x1B[14;27H\x1B[1;35m\x1B[1mOmnibrain\x1B[14;50H27.Okt.2022 22:00:23\r\n\x1B[16;10H\x1B[1;33m\x1B[1mANSi lebt!\x1B[m\r\n\x1B[1;36m\r\n\x1B[21;8HWillst du auch eine Nachricht auf der Website hinterlassen ?\r\n\x1B[22;30H\x1B[1;31m\x1B[1m<RETURN> = Abbruch\x1B[m\r\n\r\n\r\n\x1B[19;19H\x1B[1;37;44m                                    \r\n\x1B[19;19H\x1B[1m\x1B[4he\x1B[D\x1B[Pgithub\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[Punverschmte\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[Pha\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[PWerbung: gihtuh\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[Pthub.com/mkrueger/icy/\x1B[D\x1B[P_ter\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[Pgith\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[C\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\x1B[D\x1B[P\r\n\x1B[4l\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[0;1;33m\x1B[2J\r\r\n\x1B[8C\x1B[30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\r\r\n\x1B[8C\x1B[0;34m\xB0\xB0\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xB0\xB0\r\r\n\x1B[8C\xB1\xDB\xDB  \x1B[1;31m\xDC\xDC\xDC\xDC \x1B[0;34m\xDF \x1B[1;31m\xDC \x1B[0;34m\xDF \x1B[1;31m\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC\xDC \x1B[0;34m\xDF \x1B[1;31m\xDC\xDC\xDC\xDC  \x1B[0;34m\xDB\xB1\xB1\r\r\n\x1B[8C\xB2\xDB \x1B[1;31m\xDC\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB \xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF \xDB\xDB\xDB\xDB\xDB\xDB \x1B[0;34m\xDB\xB2\xB2\r\r\n\x1B[8C\xB2\xDB \x1B[1;31m\xDB\xDB\xDB\x1B[0;31m\xDB    \x1B[1m\xDB\xDB\x1B[0;31m\xDB\xDB\xDB\xDB\xDB \x1B[1m\xDB\xDB\x1B[0;31m\xDB\xDB\xDB\xDB\xDB  \x1B[1m\xDB\xDB\x1B[0;31m\xDB \x1B[1m\xDF \xDB\xDB\x1B[0;31m\xDB \x1B[1m\xDC\xDC\x1B[0;31m\xDC \x1B[1m\xDB\xDB\x1B[0;31m\xDB\xDB\xDB\xDB\xDB\xDB \x1B[1m\xDB\xDB\x1B[0;31m\xDB\xDC\xDC\xDB \x1B[0;34m\xDB\xDB\xB2\r\r\n\x1B[8C\xB2\xDB \x1B[1;31m\xDB\xDB\x1B[0;31m\xDB\xDB \xDC\xDC \x1B[1m\xDB\x1B[0;31m\xDB\xDB  \xDB\xDB \x1B[1m\xDB\x1B[0;31m\xDB\xDB \xDC \xDB  \x1B[1m\xDB\x1B[0;31m\xDB\xDB   \x1B[1m\xDB\x1B[0;31m\xDB\xDB \x1B[1m\xDB\x1B[0;31m\xDB\xDB \x1B[1m\xDB\x1B[0;31m\xDB\xDB \xDB \xDB\xDB \x1B[1m\xDB\x1B[0;31m\xDB\xDB \xDC\xDC  \x1B[0;34m\xDB\xB2\r\r\n\x1B[8C\xB2\xDB \x1B[1;31m\xDF\x1B[0;31m\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB  \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB   \xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB \x1B[0;34m\xDB\xB2\r\r\n\x1B[8C\xB1\xDB\xDB  \x1B[0;31m\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF \x1B[0;34m\xDC\xDC \x1B[0;31m\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF \x1B[0;34m\xDC \x1B[0;31m\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF \x1B[0;34m\xDC\xDC\xDC \x1B[0;31m\xDF\xDF\xDF\xDF\xDF\xDF\xDF\xDF  \x1B[0;34m\xDB\xB1\r\r\n\x1B[8C\xB0\xB0\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xDB\xB0\r\r\n\x1B[8C\x1B[1;30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\x1B7\x1B[3;70H\x1B[1;34mv\x1B[1m 1.12\x1B[m\x1B8\r\n\r\n\r\n            Sorry, der letzte FCHaT Termin war am 06.02.2022.\r\n           Ein neuer Termin wurde noch nicht vom Team angesetzt.\r\n\x1B[1;36m\x1B[80D\x1B[35C\x1B[4h\x1B[P\x1B[80D\x1B[45C\x1B[4lH\x1B[1;31m\x1B[80D\x1B[35C\x1B[4h\x1B[P\x1B[80D\x1B[45C\x1B[4li\x1B[1;36m\x1B[80D\x1B[35C\x1B[4h\x1B[P\x1B[80D\x1B[45C\x1B[4lT\x1B[1;30m\x1B[80D\x1B[35C\x1B[4h\x1B[P\x1B[80D\x1B[45C\x1B[4l \x1B[1;31m\x1B[80D\x1B[35C\x1B[4h\x1B[P\x1B[80D\x1B[45C\x1B[4la\x1B[1;36m\x1B[80D\x1B[35C\x1B[4h\x1B[P\x1B[80D\x1B[45C\x1B[4lN\r\n\x1B[m\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[1;31mKE\x1B[1mi\x1B[m\x1B[1;31mNE N\x1B[1me\x1B[m\x1B[1;31mUEN PM\x1B[1ms\x1B[m\x1B[1;31m G\x1B[1me\x1B[m\x1B[1;31mFUND\x1B[1me\x1B[m\x1B[1;31mN\x1B[m  !\r\n\x1B[1;35mKE\x1B[1mi\x1B[m\x1B[1;35mNE N\x1B[1ma\x1B[m\x1B[1;35mCHR\x1B[1mi\x1B[m\x1B[1;35mCHTEN G\x1B[1me\x1B[m\x1B[1;35mFUND\x1B[1me\x1B[m\x1B[1;35mN\x1B[m\x1B[1m!\x1B[m\r\n\r\n\x1B[1;36mBR\x1B[1me\x1B[m\x1B[1;36mTTN\x1B[1ma\x1B[m\x1B[1;36mME\x1B[59CF\x1B[1mi\x1B[m\x1B[1;36mLE\x1B[1ma\x1B[m\x1B[1;36mNZ\x1B[1ma\x1B[m\x1B[1;36mHL\x1B[m\r\n\x1B[1;30m\x1B[1m\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\x1B[m\r\n\x1B[1;33mFastnet/Files/\x1B[1mPC\r\x1B[74C\x1B[1;32m   1\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1m3.1_Updates\r\x1B[74C\x1B[1;32m   6\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1m3.5_Updates\r\x1B[74C\x1B[1;32m   4\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1m3.9_Updates\r\x1B[74C\x1B[1;32m  30\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1mFonts\r\x1B[74C\x1B[1;32m  40\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1mIcons\r\x1B[74C\x1B[1;32m   1\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1mInternet\r\x1B[74C\x1B[1;32m  40\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1mSound\r\x1B[74C\x1B[1;32m  52\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1mTerminals\r\x1B[74C\x1B[1;32m  10\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1mTools&Utilities\r\x1B[74C\x1B[1;32m  18\x1B[m\r\n\x1B[1;33mFiles/Amiga/\x1B[1mTreiber\r\x1B[74C\x1B[1;32m  19\x1B[m\r\n\x1B[1;33mFiles/BBS-SySTEME/\x1B[1mBBS-PC\r\x1B[74C\x1B[1;32m 153\x1B[m\r\n\x1B[1;33mFiles/BBS-SySTEME/\x1B[1mPC-Clients\r\x1B[74C\x1B[1;32m   2\x1B[m\r\n\x1B[1;33mFiles/BBS-SySTEME/AMiGA/\x1B[1mAMBoS\r\x1B[74C\x1B[1;32m 163\x1B[m\r\n\x1B[1;33mFiles/BBS-SySTEME/AMiGA/\x1B[1mAmiExpress\r\x1B[74C\x1B[1;32m  87\x1B[m\r\n\x1B[1;33mFiles/C64/\x1B[1mBBS\r\x1B[74C\x1B[1;32m  12\x1B[m\r\n\x1B[1;33mFiles/Grafik/\x1B[1mBilder-Sonstige\r\x1B[74C\x1B[1;32m  12\x1B[m\r\n\x1B[1;33mFiles/Grafik/\x1B[1mBilder-User\r\x1B[74C\x1B[1;32m   1\x1B[m\r\n\x1B[1;33mFiles/Grafik/\x1B[1mEric_Schwartz\r\x1B[74C\x1B[1;32m  83\x1B[m\r\n\x1B[1;33mFiles/Windows/\x1B[1mDeMoS\r\x1B[74C\x1B[1;32m  10\x1B[m\r\n\x1B[1;33mSystem/\x1B[1mPrivat\r\x1B[74C\x1B[1;32m  10\x1B[m\r\n---   WEiTER   ---\r\x1B[2K\x1B[1;30m\x1B[1m\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\xCD\x1B[m\r\n\x1B[63C\x1B[1;33mS\x1B[1mu\x1B[m\x1B[1;33mMME\x1B[m :\x1B[1;32m\x1B[1m     754\x1B[m\r\n\r\n---   WEiTER   ---\r\x1B[2KList Pmsgs 2.12b (c) bY kLOSAU: sEARCHING pMSGS .. 4 PMSGS fOUND !\r\nPMSG von \x1B[1;31mIRoN                \x1B[m gefunden ! Abgesandt am  3.11.22 um 21:55:54 !\r\nPMSG von \x1B[1;31mIRoN                \x1B[m gefunden ! Abgesandt am  3.11.22 um 21:52:47 !\r\nPMSG von \x1B[1;31mIRoN                \x1B[m gefunden ! Abgesandt am  3.11.22 um 21:53:49 !\r\nPMSG von \x1B[1;31mIRoN                \x1B[m gefunden ! Abgesandt am  3.11.22 um 21:55:23 !\r\n\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[0;1;40;30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[30m\xF5\xF5\x1B[0m\x1B[11C\x1B[1;30m\xF5\xF5 \xDC\xDB\xDF \x1B[32m\xDC\xDC\xDC\xDC\xDC   \xDC\xDC\xDC   \xDC\xDC\xDC\xDC\xDC   \xDC    \xDC  \xDC\xDC\xDC \x1B[30m\xDF\xDB\xDC \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDC\xDB\x1B[0m\xDB \x1B[1;32m\xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB \x1B[30m\xFA \x1B[32m\xDB \xDB\xDB\xDB\xDB\xDB \x1B[0m\xDB\x1B[1;30m\xDB\xDC \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDC\xDB\x1B[0m\xDB\x1B[1m\xDB \x1B[0;32m\xDB\x1B[1m\xDB\xDB \xDB \xDB \xDB\xDB\xDB \xDF \x1B[0;32m\xDB\x1B[1m\xDB\xDB   \xDB \x1B[0;32m\xDB\x1B[1m\xDB\xDB   \xDB \x1B[0;32m\xDB\x1B[1m\xDB\xDB \xDF \x1B[37m\xDB\x1B[0m\xDB\x1B[1;30m\xDB\xDC \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDB\x1B[0m\xDB\x1B[1m\xDB  \x1B[0;32m\xDB\xDB\xDB   \xDB \xDB\x1B[1m\xDB  \xDB \x1B[0;32m\xDB\xDB\xDB   \xDB \xDB\xDB\x1B[1m\xDB\xDC\xDC \x1B[0;32m\xDB \xDB\xDB  \x1B[1m\xDB  \x1B[37m\xDB\x1B[0m\xDB\x1B[1;30m\xDB \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDF\xDB\x1B[0m\xDB\x1B[1m\xDB \x1B[0;32m\xDB\xDB\xDB \x1B[1;30m\xFA \x1B[0;32m\xDB \xDB\xDB\xDB \xDC \xDB\xDB\xDB \x1B[1;30m\xFA \x1B[0;32m\xDB \xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB \xDC \x1B[1;37m\xDB\x1B[0m\xDB\x1B[1;30m\xDB\xDF \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDF\xDB\x1B[0m\xDB\x1B[1m\xDC \x1B[0;32m\xDF    \xDF \xDB\xDB\xDB\xDB\xDB  \xDF    \xDF  \xDF\xDF\xDF\xDF\xDF  \xDB\xDB\xDB\xDB\xDB \x1B[37m\xDB\x1B[1;30m\xDB\xDF \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDF\xDB\xDB\xDC\xDC\xDC\xDC\xDB \x1B[0;32m\xDB  \xDF\xDF\xDF  \x1B[1;30m\xDB\xDC\xDC\xDC\xDB \x1B[0;32m\xDB \x1B[1;30m\xDB\xDC\xDC\xDC\xDC\xDC\xDB  \x1B[0;32m\xDF\xDF\xDF \x1B[1;30m\xDC\xDB\xDF \xF5\xF5\x1B[0;35mdESiGN bY \x1B[1mMD \x1B[0;35m'2007\x1B[1;30m\xF5\xF5\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\r\n\r\n\r\n\r\n\r\n\r\n\r\n\r\n\r\n\r\n\x1B[30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4  \x1B[34mM\x1B[0;34mENU - \x1B[1mi\x1B[0;34mNFO \x1B[36m:\r\n  \x1B[1;34mT\x1B[0;34mEAM - \x1B[1mi\x1B[0;34mNFO \x1B[36m:\r\n\x1B[1;30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[2;3H\x1B[1mHaUPTMeNU\x1B[m\r\n\x1B[10;10H\x1B[36m[\x1B[33mW\x1B[36m] \x1B[32mSHoW PoRTS         \r\n\x1B[11;10H\x1B[36m[\x1B[33mU\x1B[36m] \x1B[32mUsER EiNSTeLLuNGEN \r\n\x1B[12;10H\x1B[36m[\x1B[33mS\x1B[36m] \x1B[32mSTaTISTiKEN        \r\n\x1B[13;10H\x1B[36m[\x1B[33mI\x1B[36m] \x1B[32mSySTEM INFoS       \r\n\x1B[14;10H\x1B[36m[\x1B[33mA\x1B[36m] \x1B[32mAnTRAGSMEnU        \r\n\x1B[15;10H\x1B[36m[\x1B[33mO\x1B[36m] \x1B[32moNLiNE SPiELE      \r\n\x1B[10;45H\x1B[36m[\x1B[33mF\x1B[36m] \x1B[32mFiLE MeNU          \r\n\x1B[11;45H\x1B[36m[\x1B[33mM\x1B[36m] \x1B[32mMeSSAGE MeNU       \r\n\x1B[12;45H\x1B[36m[\x1B[33mP\x1B[36m] \x1B[32mMAiL MeNU          \r\n\x1B[13;45H\x1B[36m[\x1B[33mC\x1B[36m] \x1B[32mCHaT MEnU          \r\n\x1B[14;45H\x1B[36m[\x1B[33mB\x1B[36m] \x1B[32mBeFEHLSeBENE       \r\n\x1B[15;45H\x1B[36m[\x1B[33mL\x1B[36m] \x1B[32mLoGOUT             \r\n\x1B[m\x1B[11;10H\x1B[36m[\x1B[33mU\x1B[36m] \x1B[32mUsER EiNSTeLLuNGEN \x1B[1m\x1B[1;32m\x1B[10;10H\x1B[36m[\x1B[33mW\x1B[36m] \x1B[32mSHoW PoRTS         \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[K0 aNDeRE uSER SiND NoCH iM SySTEM\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:51\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[10;10H\x1B[36m[\x1B[33mW\x1B[36m] \x1B[32mSHoW PoRTS         \x1B[1m\x1B[1;32m\x1B[11;10H\x1B[36m[\x1B[33mU\x1B[36m] \x1B[32mUsER EiNSTeLLuNGEN \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[KHiER kANNsT dU DEiNE PERSoENLiCHEN EiNSTeLLUNGeN fESTLeGEN\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:52\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[11;10H\x1B[36m[\x1B[33mU\x1B[36m] \x1B[32mUsER EiNSTeLLuNGEN \x1B[1m\x1B[1;32m\x1B[11;45H\x1B[36m[\x1B[33mM\x1B[36m] \x1B[32mMeSSAGE MeNU       \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[K0 NeUE NaCHRICHtEN HaST dU zUM LeSEN\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:53\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[11;45H\x1B[36m[\x1B[33mM\x1B[36m] \x1B[32mMeSSAGE MeNU       \x1B[1m\x1B[1;32m\x1B[12;45H\x1B[36m[\x1B[33mP\x1B[36m] \x1B[32mMAiL MeNU          \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[K0 NeUE MaILS WaRTEN iN DEiN PoSTFACH\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:54\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[12;45H\x1B[36m[\x1B[33mP\x1B[36m] \x1B[32mMAiL MeNU          \x1B[1m\x1B[1;32m\x1B[13;45H\x1B[36m[\x1B[33mC\x1B[36m] \x1B[32mCHaT MEnU          \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[KDaS KoMMuNiKATiONS MeNU MiT dEM bELieBTEN FCHAT !\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:54\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[13;45H\x1B[36m[\x1B[33mC\x1B[36m] \x1B[32mCHaT MEnU          \x1B[1m\x1B[1;32m\x1B[14;45H\x1B[36m[\x1B[33mB\x1B[36m] \x1B[32mBeFEHLSeBENE       \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[KFuER HArDCoRE UsER die BEFeHLSeBENE\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:54\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[14;45H\x1B[36m[\x1B[33mB\x1B[36m] \x1B[32mBeFEHLSeBENE       \x1B[1m\x1B[1;32m\x1B[15;45H\x1B[36m[\x1B[33mL\x1B[36m] \x1B[32mLoGOUT             \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[KnA dENN ... RiNNjEHAUN !\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:54\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[15;45H\x1B[36m[\x1B[33mL\x1B[36m] \x1B[32mLoGOUT             \x1B[1m\x1B[1;32m\x1B[14;45H\x1B[36m[\x1B[33mB\x1B[36m] \x1B[32mBeFEHLSeBENE       \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[KFuER HArDCoRE UsER die BEFeHLSeBENE\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:55\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[14;45H\x1B[36m[\x1B[33mB\x1B[36m] \x1B[32mBeFEHLSeBENE       \x1B[1m\x1B[1;32m\x1B[13;45H\x1B[36m[\x1B[33mC\x1B[36m] \x1B[32mCHaT MEnU          \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[KDaS KoMMuNiKATiONS MeNU MiT dEM bELieBTEN FCHAT !\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:55\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[13;45H\x1B[36m[\x1B[33mC\x1B[36m] \x1B[32mCHaT MEnU          \x1B[1m\x1B[1;32m\x1B[12;45H\x1B[36m[\x1B[33mP\x1B[36m] \x1B[32mMAiL MeNU          \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[K0 NeUE MaILS WaRTEN iN DEiN PoSTFACH\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:55\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[m\x1B[12;45H\x1B[36m[\x1B[33mP\x1B[36m] \x1B[32mMAiL MeNU          \x1B[1m\x1B[1;32m\x1B[11;45H\x1B[36m[\x1B[33mM\x1B[36m] \x1B[32mMeSSAGE MeNU       \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H\x1B[1m\x1B[K0 NeUE NaCHRICHtEN HaST dU zUM LeSEN\x1B[m\r\n\x1B[21;17H\x1B[1m\x1B[KPMSGS Vorhanden\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:55\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[1;31m\r\n\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[1;25r\x1B[4l\x1B[m\x1B[H\x1B[2J\x1B[0;1;40;30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[30m\xF5\xF5\x1B[0m\x1B[11C\x1B[1;30m\xF5\xF5 \xDC\xDB\xDF \x1B[32m\xDC\xDC\xDC\xDC\xDC   \xDC\xDC\xDC   \xDC\xDC\xDC\xDC\xDC   \xDC    \xDC  \xDC\xDC\xDC \x1B[30m\xDF\xDB\xDC \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDC\xDB\x1B[0m\xDB \x1B[1;32m\xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB \x1B[30m\xFA \x1B[32m\xDB \xDB\xDB\xDB\xDB\xDB \x1B[0m\xDB\x1B[1;30m\xDB\xDC \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDC\xDB\x1B[0m\xDB\x1B[1m\xDB \x1B[0;32m\xDB\x1B[1m\xDB\xDB \xDB \xDB \xDB\xDB\xDB \xDF \x1B[0;32m\xDB\x1B[1m\xDB\xDB   \xDB \x1B[0;32m\xDB\x1B[1m\xDB\xDB   \xDB \x1B[0;32m\xDB\x1B[1m\xDB\xDB \xDF \x1B[37m\xDB\x1B[0m\xDB\x1B[1;30m\xDB\xDC \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDB\x1B[0m\xDB\x1B[1m\xDB  \x1B[0;32m\xDB\xDB\xDB   \xDB \xDB\x1B[1m\xDB  \xDB \x1B[0;32m\xDB\xDB\xDB   \xDB \xDB\xDB\x1B[1m\xDB\xDC\xDC \x1B[0;32m\xDB \xDB\xDB  \x1B[1m\xDB  \x1B[37m\xDB\x1B[0m\xDB\x1B[1;30m\xDB \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDF\xDB\x1B[0m\xDB\x1B[1m\xDB \x1B[0;32m\xDB\xDB\xDB \x1B[1;30m\xFA \x1B[0;32m\xDB \xDB\xDB\xDB \xDC \xDB\xDB\xDB \x1B[1;30m\xFA \x1B[0;32m\xDB \xDB\xDB\xDB\xDB\xDB\xDB\xDB \xDB\xDB\xDB \xDC \x1B[1;37m\xDB\x1B[0m\xDB\x1B[1;30m\xDB\xDF \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDF\xDB\x1B[0m\xDB\x1B[1m\xDC \x1B[0;32m\xDF    \xDF \xDB\xDB\xDB\xDB\xDB  \xDF    \xDF  \xDF\xDF\xDF\xDF\xDF  \xDB\xDB\xDB\xDB\xDB \x1B[37m\xDB\x1B[1;30m\xDB\xDF \xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5\xF5 \xDF\xDB\xDB\xDC\xDC\xDC\xDC\xDB \x1B[0;32m\xDB  \xDF\xDF\xDF  \x1B[1;30m\xDB\xDC\xDC\xDC\xDB \x1B[0;32m\xDB \x1B[1;30m\xDB\xDC\xDC\xDC\xDC\xDC\xDB  \x1B[0;32m\xDF\xDF\xDF \x1B[1;30m\xDC\xDB\xDF \xF5\xF5\x1B[0;35mdESiGN bY \x1B[1mMD \x1B[0;35m'2007\x1B[1;30m\xF5\xF5\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\r\n\r\n\r\n\r\n\r\n\r\n\r\n\r\n\r\n\r\n\x1B[30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4  \x1B[34mM\x1B[0;34mENU - \x1B[1mi\x1B[0;34mNFO \x1B[36m:\r\n  \x1B[1;34mT\x1B[0;34mEAM - \x1B[1mi\x1B[0;34mNFO \x1B[36m:\r\n\x1B[1;30m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[0m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[1m\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\xC4\x1B[2;3H\x1B[1mMsG-MeNU\x1B[m\r\n\x1B[10;10H\x1B[36m[\x1B[33mN\x1B[36m] \x1B[32mNeUE Nachrichten   \r\n\x1B[11;10H\x1B[36m[\x1B[33mA\x1B[36m] \x1B[32mAnZaHL NeUER MsGS  \r\n\x1B[12;10H\x1B[36m[\x1B[33mL\x1B[36m] \x1B[32mBReTTLiSTE AeNDeRN \r\n\x1B[13;10H\x1B[36m[\x1B[33mS\x1B[36m] \x1B[32mBReTTDiREKTaNWaHL 1\r\n\x1B[14;10H\x1B[36m[\x1B[33mV\x1B[36m] \x1B[32mBReTTDiREKTaNWaHL 2\r\n\x1B[15;10H\x1B[36m[\x1B[33mB\x1B[36m] \x1B[32mBReTTLiSTE AnSEHeN \r\n\x1B[10;45H\x1B[36m[\x1B[33mE\x1B[36m] \x1B[32mMsGs ZuRUeCKSeTZeN \r\n\x1B[11;45H\x1B[36m[\x1B[33mD\x1B[36m] \x1B[32mFeSTPLaTTENSPEiCHER\r\n\x1B[12;45H\x1B[36m[\x1B[33mQ\x1B[36m] \x1B[32mHaUPTMeNU          \r\n\x1B[m\x1B[11;10H\x1B[36m[\x1B[33mA\x1B[36m] \x1B[32mAnZaHL NeUER MsGS  \x1B[1m\x1B[1;32m\x1B[10;10H\x1B[36m[\x1B[33mN\x1B[36m] \x1B[32mNeUE Nachrichten   \x1B[1;31m \x1B[24D>\x1B[C<\x1B[m\x1B[20;17H                                                              \r\n\x1B[21;17H                                                              \r\n\x1B[20;17H\x1B[1m0 NeUE NaCHRiCHTeN LeSEN\x1B[m\r\n\x1B[21;17H\x1B[1m- - - -\x1B[m\r\n\x1B[4;64H3.Nov.2022\x1B[5;66H22:08:56\r\n\x1B[H\x1B[17;35H->?<-\x1B[3D\x1B[4l\x1B[m\x1B[1;33m\r\nPMSG(s) eMPFaNGeN ! \x1B[1;36mRETURN zUM LeSEN oDeR CTRL-O fUER SPAeTER LeSEN dRUeCKEN !\x1B[m";
        for b in txt {
            if let Err(err) = view.buffer_view.buffer_parser.print_char(&mut view.buffer_view.buf, &mut view.buffer_view.caret, *b) {
                eprintln!("{}", err);
            }
        }*/

        (view, Command::none())
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.trigger = !self.trigger;

        if unsafe { READ_ADDRESSES } {
            unsafe { READ_ADDRESSES = false; } 
            self.addresses = Address::read_phone_book();
        }

        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let in_ms = since_the_epoch.as_millis();
        
        if in_ms - self.buffer_view.last_blink > 550 {
            self.buffer_view.blink = !self.buffer_view.blink;
            self.buffer_view.last_blink = in_ms;
        }
        
        match &message {
            Message::OpenURL(url) => {
                open::that(url);
            }
            _ => {}
        };

        match self.mode {
            MainWindowMode::Default => {
                match message {
                    Message::InitiateFileTransfer(download)=> {
                        self.mode = MainWindowMode::SelectProtocol(download);
                    },
                    Message::SendLogin => {
                        if let Some(com) = &mut self.com {
                            let adr = self.addresses.get(self.cur_addr).unwrap();
                            if let Err(err) = com.write([adr.user_name.as_bytes(), b"\r", adr.password.as_bytes(), b"\r"].concat().as_slice()) {
                                eprintln!("Error sending login: {}", err);
                                self.print_log(format!("Error sending login: {}", err));
                            }
                            self.auto_login.logged_in = true;
                        }
                    }
                    Message::ShowPhonebook => {
                        self.mode = MainWindowMode::ShowPhonebook;
                    },
                    Message::Hangup => {
                        self.com = None;
                        self.print_log(format!("Disconnected."));

                    },
                    Message::Tick => { 
                        let state = self.update_state(); 

                        if let Err(err) = state {
                            eprintln!("{}", err);
                            self.print_log(format!("Error: {:?}", err));
                        }
                    },
                    Message::KeyPressed(ch) => {
                        let c = ch as u8;

                        if c == 0x13 || c == 0x14 {
                            println!();
                        }
                        if c != 8 && c != 127 { // handled by key
                            self.output_char(ch);
                        }
                    },
                    Message::KeyCode(code, _modifier) => {
                        
                        if let Some(com) = &mut self.com {
                            for (k, m) in if self.buffer_view.petscii { C64_KEY_MAP} else { KEY_MAP } {
                                if *k == code {
                                    let state = com.write(m);
                                    if let Err(err) = state {
                                        eprintln!("{}", err);
                                        self.print_log(format!("Error: {:?}", err));
                                        self.com = None;
                                    }
                                    break;
                                }
                            }
                        } else {
                            for (k, m) in if self.buffer_view.petscii { C64_KEY_MAP} else { KEY_MAP } {
                                if *k == code {
                                    for ch in *m {
                                        let state = self.buffer_view.print_char::<TelnetCom>(Option::<&mut TelnetCom>::None, *ch);
                                        if let Err(s) = state {
                                            self.print_log(format!("Error: {:?}", s));
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    Message::WheelScrolled(delta) => {
                        if let ScrollDelta::Lines { y, .. } = delta {
                            self.buffer_view.scroll(y as i32);
                            self.buffer_view.cache.clear();
                        }
                    }
                    Message::FontSelected(font) => {
                        self.set_font(&font);
                    }
                    Message::ScreenModeSelected(mode) => {
                        self.set_screen_mode(&mode);
                    }
                    _ => {}
                }

            },
            MainWindowMode::ShowPhonebook => {
                text_input::focus::<Message>(super::INPUT_ID.clone());
                match message {
                    Message::ShowPhonebook => {
                        self.mode = MainWindowMode::ShowPhonebook
                    },
                    Message::Back => {
                        self.mode = MainWindowMode::Default
                    },

                    Message::EditBBS(i) => {
                        self.edit_bbs = if i == 0 { Address::new() } else { self.addresses[i].clone() };
                        self.mode = MainWindowMode::EditBBS(i)
                    }
                    
                    Message::CallBBS(i) => {
                        self.mode = MainWindowMode::Default;
                        let mut adr = self.addresses[i].address.clone();
                        if !adr.contains(":") {
                            adr.push_str(":23");
                        }
                        self.print_log(format!("Connect to {}…", adr));
                        let mut socket_addr = adr.to_socket_addrs();
                        match &mut socket_addr {
                            Ok(socket_addr) => {
                                if let Some(addr) = &socket_addr.next() {
                                    let t = TelnetCom::connect(addr, self.options.connect_timeout);
                                    match t {
                                        Ok(t) => {
                                            self.buffer_view.clear();
                                            self.com = Some(t);
                                            self.cur_addr = i;
                                            self.connection_time = SystemTime::now();
                                            let adr = self.addresses[i].clone();
                                            self.auto_login = AutoLogin::new(adr.auto_login);

                                            self.buffer_view.buf.clear();
                                            if let Some(mode) = &adr.screen_mode {
                                                self.set_screen_mode(mode);
                                            } else {
                                                self.set_screen_mode(&ScreenMode::DOS(80, 25));
                                            }
                                            if let Some(font) = &adr.font_name {
                                                self.set_font(font);
                                            }
                                            self.buffer_view.buffer_parser = self.addresses[i].get_terminal_parser();
                                        },
                                        Err(err) => {
                                            eprintln!("{}", err);
                                            self.print_log(format!("Error: {:?}", err));
                                        }
                                    }
                                }
                            }
                            Err(error) => {
                                eprintln!("{}", error);
                                self.print_log(format!("Socket error: {:?}", error));
                            }
                        }
                    },

                    Message::QuickConnectChanged(addr) => {
                        self.addresses[0].address = addr
                    }
                    _ => {}
                }
            },
            MainWindowMode::SelectProtocol(_) => {
                match message {
                    Message::Back => {
                        self.mode = MainWindowMode::Default
                    }
                    Message::SelectProtocol(protocol_type, download) => {
                        self.mode = MainWindowMode::Default;
                        if let Some(com) = self.com.as_mut() {
                            if !download {
                                    let files = FileDialog::new()
                                        .pick_files();
                                    if let Some(path) = files {
                                        let fd = FileDescriptor::from_paths(&path);
                                        if let Ok(files) =  fd {
                                                let r = match protocol_type {
                                                    ProtocolType::ZModem => {
                                                        self.zmodem = Zmodem::new(1024);
                                                        self.zmodem.initiate_send(com, files)
                                                    },
                                                    ProtocolType::ZedZap => {
                                                        self.zmodem = Zmodem::new(8 * 1024);
                                                        self.zmodem.initiate_send(com, files)
                                                    },
                                                    ProtocolType::XModem => {
                                                        self.xymodem = XYmodem::new(XYModemVariant::XModem);
                                                        self.xymodem.initiate_send(com, files)
                                                    },
                                                    ProtocolType::XModem1k => {
                                                        self.xymodem = XYmodem::new(XYModemVariant::XModem1k);
                                                        self.xymodem.initiate_send(com, files)
                                                    },
                                                    ProtocolType::XModem1kG => {
                                                        self.xymodem = XYmodem::new(XYModemVariant::XModem1kG);
                                                        self.xymodem.initiate_send(com, files)
                                                    },
                                                    ProtocolType::YModem => {
                                                        self.xymodem = XYmodem::new(XYModemVariant::YModem);
                                                        self.xymodem.initiate_send(com, files)
                                                    },
                                                    ProtocolType::YModemG => {
                                                        self.xymodem = XYmodem::new(XYModemVariant::YModemG);
                                                        self.xymodem.initiate_send(com, files)
                                                    }
                                                };
                                                self.print_result(&r);
                                                if r.is_ok() {
                                                    self.mode = MainWindowMode::FileTransfer(protocol_type, download);
                                                }
                                        } else {
                                            self.print_result(&fd);
                                        }
                                    } 
                            } else {
                                if let Some(com) = self.com.as_mut() {
                                    let r = match protocol_type {
                                        ProtocolType::ZModem => {
                                            self.zmodem.initiate_recv(com)
                                        },
                                        ProtocolType::ZedZap => {
                                            self.zmodem.initiate_recv(com)
                                        },
                                        ProtocolType::XModem => {
                                            self.xymodem = XYmodem::new(XYModemVariant::XModem);
                                            self.xymodem.initiate_recv(com)
                                        },
                                        ProtocolType::XModem1k => {
                                            self.xymodem = XYmodem::new(XYModemVariant::XModem1k);
                                            self.xymodem.initiate_recv(com)
                                        },
                                        ProtocolType::XModem1kG => {
                                            self.xymodem = XYmodem::new(XYModemVariant::XModem1kG);
                                            self.xymodem.initiate_recv(com)
                                        },
                                        ProtocolType::YModem => {
                                            self.xymodem = XYmodem::new(XYModemVariant::YModem);
                                            self.xymodem.initiate_recv(com)
                                        },
                                        ProtocolType::YModemG => {
                                            self.xymodem = XYmodem::new(XYModemVariant::YModemG);
                                            self.xymodem.initiate_recv(com)
                                        }
                                    };
                                    self.print_result(&r);
                                    if r.is_ok() {
                                        self.mode = MainWindowMode::FileTransfer(protocol_type, download);
                                    }
                                } else {
                                    self.print_log("Communication error.".to_string());
                                }

                            }
                        } else {
                            self.print_log("Communication error.".to_string());
                        }
                    }
                    _ => { }
                }
            }
            MainWindowMode::FileTransfer(protocol_type, _download) => {
                match message {
                    Message::Tick => { 
                        if let Some(com) = &mut self.com {
                            let r = match protocol_type {
                                ProtocolType::ZModem | ProtocolType::ZedZap => {
                                    self.zmodem.update(com)
                                },
                                _ => {
                                    self.xymodem.update(com)
                                }
                            };
                            self.print_result(&r);

                            if !self.zmodem.is_active() && !self.xymodem.is_active() {
                                for f in self.zmodem.get_received_files() {
                                    f.save_file_in_downloads().expect("error saving file.");
                                }
                                for f in self.xymodem.get_received_files() {
                                    f.save_file_in_downloads().expect("error saving file.");
                                }
                                self.mode = MainWindowMode::Default;
                            }
                        }
                    },
                    Message::CancelTransfer => {
                        if let Some(com) = &mut self.com {
                            if self.zmodem.is_active() {
                                let r = self.zmodem.cancel(com);
                                if let Err(err) = r {
                                    eprintln!("{}", err);
                                    println!("Error while cancel {:?}", err);
                                }
                            }
                            if self.xymodem.is_active() {
                                let r = self.xymodem.cancel(com);
                                if let Err(err) = r {
                                    eprintln!("{}", err);
                                    println!("Error while cancel {:?}", err);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            MainWindowMode::EditBBS(_) => {
                text_input::focus::<Message>(super::INPUT_ID.clone());
                match message {
                    Message::Back => {
                        self.mode = MainWindowMode::ShowPhonebook;
                    },

                    Message::EditBbsSystemNameChanged(str) => self.edit_bbs.system_name = str,
                    Message::EditBbsAddressChanged(str) => self.edit_bbs.address = str,
                    Message::EditBbsUserNameChanged(str) => self.edit_bbs.user_name = str,
                    Message::EditBbsPasswordChanged(str) => self.edit_bbs.password = str,
                    Message::EditBbsCommentChanged(str) => self.edit_bbs.comment = str,
                    Message::EditBbsTerminalTypeSelected(terminal) => self.edit_bbs.terminal_type = terminal,
                    Message::EditBbsScreenModeSelected(screen_mode) => self.edit_bbs.screen_mode = Some(screen_mode),
                    Message::EditBbsAutoLoginChanged(str) => self.edit_bbs.auto_login = str,
                    
                    Message::EditBbsSaveChanges(i) => {
                        if i == 0 { 
                            self.addresses.push(self.edit_bbs.clone());
                        } else {
                            self.addresses[i] = self.edit_bbs.clone();
                        }
                        self.print_result(&store_phone_book(&self.addresses));
                        self.mode = MainWindowMode::ShowPhonebook;
                    }
                    Message::EditBbsDeleteEntry(i) => {
                        if i > 0 { 
                            self.addresses.remove(i);
                        }
                        self.print_result(&store_phone_book(&self.addresses));
                        self.mode = MainWindowMode::ShowPhonebook;
                    }
                    _ => {}
                }
            }

        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        
        let s = subscription::events_with(|event, status| match (event, status) {
            (Event::Keyboard(keyboard::Event::CharacterReceived(ch)), iced::event::Status::Ignored) => Some(Message::KeyPressed(ch)),
            (Event::Keyboard(keyboard::Event::KeyPressed {key_code, modifiers, ..}), iced::event::Status::Ignored) => Some(Message::KeyCode(key_code, modifiers)),
            (Event::Mouse(mouse::Event::WheelScrolled {delta, ..}), iced::event::Status::Ignored) => Some(Message::WheelScrolled(delta)),
            _ => None,
        });

        let t = iced::time::every(std::time::Duration::from_millis(10))
            .map(|_| Message::Tick);

        Subscription::<Message>::batch([s, t])
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        
        match self.mode {
            MainWindowMode::Default => {
                let c = Canvas::new(&self.buffer_view)
                    .width(Length::Fill)
                    .height(Length::Fill);
                
                let log_info = if self.log_file.len() == 0  { text("Ready.")} else { text(&self.log_file[self.log_file.len() - 1])}.width(Length::Fill).into();
                let all_fonts = SUPPORTED_FONTS.map(|s| s.to_string()).to_vec();
                let font_pick_list = pick_list(
                    all_fonts,
                    Some(self.get_font_name()),
                    Message::FontSelected
                );

                let screen_mode_pick_list: iced_native::widget::pick_list::PickList<'_, ScreenMode, Message, iced::Renderer> = pick_list(
                    DEFAULT_MODES.to_vec(),
                    Some(self.get_screen_mode()),
                    Message::ScreenModeSelected
                );

                column(vec![
                    if !self.auto_login.logged_in && self.com.is_some() && self.addresses[self.cur_addr].user_name.len() > 0 {
                        row![
                            button("Phonebook")
                                .on_press(Message::ShowPhonebook),
                            button("Upload")
                                .on_press(Message::InitiateFileTransfer(false)),
                            button("Download")
                                .on_press(Message::InitiateFileTransfer(true)),
                            button("Send login")
                                .on_press(Message::SendLogin),
                                button("Hangup")
                                .on_press(Message::Hangup)
                        ]
                    } else {
                        if  self.com.is_some()  {
                            row![
                                button("Phonebook")
                                    .on_press(Message::ShowPhonebook),
                                button("Upload")
                                    .on_press(Message::InitiateFileTransfer(false)),
                                button("Download")
                                    .on_press(Message::InitiateFileTransfer(true)),
                                button("Hangup")                            
                                    .on_press(Message::Hangup) 
                            ]
                        } else {
                            row![
                                button("Phonebook")
                                    .on_press(Message::ShowPhonebook),
                                button("Upload")
                                    .on_press(Message::InitiateFileTransfer(false)),
                                button("Download")
                                    .on_press(Message::InitiateFileTransfer(true)),
                            ]
                        }
                    }.padding(10).spacing(20).into(),
                    c.into(),
                    if self.com.is_none() {
                        row(vec![
                            log_info,
                            vertical_rule(10).into(),
                            text("Offline").into(),
                            vertical_rule(10).into(),
                            font_pick_list.into(),
                            screen_mode_pick_list.into(),
                        ])
                    } else {
                        let d = SystemTime::now().duration_since(self.connection_time).unwrap();
                        let sec     = d.as_secs();
                        let minutes = sec / 60;
                        let hours   = minutes  / 60;
                        let cur = &self.addresses[self.cur_addr];

                        row(vec![
                            log_info,
                            vertical_rule(10).into(),
                            text(if cur.system_name.len() > 0 { &cur.system_name } else { &cur.address }).into(),
                            vertical_rule(10).into(),
                            text(format!("Connected {:02}:{:02}:{:02}", hours, minutes % 60, sec % 60)).into(),
                            vertical_rule(10).into(),
                            font_pick_list.into(),
                            screen_mode_pick_list.into(),
                        ])
                    }
                    .padding(8)
                    .spacing(20)
                    .height(Length::Units(40))
                    .align_items(Alignment::Start)
                    .into()
                ]).spacing(8)
                .into()
            }
            MainWindowMode::ShowPhonebook => {   
                super::view_phonebook(self)            
            }
            MainWindowMode::SelectProtocol(download) => {   
                super::view_protocol_selector(download)
            }
            MainWindowMode::EditBBS(i) => {
                super::view_edit_bbs(self, &self.edit_bbs, i)
            }
            MainWindowMode::FileTransfer(protocol_type, download) => {
                match protocol_type {
                    ProtocolType::ZModem => {
                        super::view_file_transfer(&self.zmodem, download)
                    },
                    ProtocolType::ZedZap => {
                        super::view_file_transfer(&self.zmodem, download)
                    },
                    _ => {
                        super::view_file_transfer(&self.xymodem, download)
                    }
                }
            }
        }
    }
}

