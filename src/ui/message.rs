use iced::{keyboard::{KeyCode, Modifiers}, mouse::ScrollDelta};

use crate::{protocol::ProtocolType, address::Terminal};

use super::screen_modes::ScreenMode;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    InitiateFileTransfer(bool),
    SendLogin,
    Back,
    Hangup,
    KeyPressed(char),
    KeyCode(KeyCode, Modifiers),
    WheelScrolled(ScrollDelta),
    FontSelected(String),
    ScreenModeSelected(ScreenMode),
    SelectProtocol(ProtocolType, bool),
    OpenURL(String),
    CancelTransfer,

    // Phonebook
    ShowPhonebook,
    QuickConnectChanged(String),
    CallBBS(usize),

    // Edit BBS 
    EditBBS(usize),
    EditBbsSystemNameChanged(String),
    EditBbsAddressChanged(String),
    EditBbsUserNameChanged(String),
    EditBbsPasswordChanged(String),
    EditBbsCommentChanged(String),
    EditBbsTerminalTypeSelected(Terminal),
    EditBbsScreenModeSelected(ScreenMode),
    EditBbsAutoLoginChanged(String),
    EditBbsSaveChanges(usize),
    EditBbsDeleteEntry(usize)
}