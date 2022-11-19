use iced::{
    keyboard::{KeyCode, Modifiers},
    mouse::ScrollDelta,
};

use crate::{
    address::{ConnectionType, Terminal},
    protocol::ProtocolType,
};

use super::{screen_modes::ScreenMode, selection::Selection, HoverListMessage};

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    InitiateFileTransfer(bool),
    SendLogin,
    Connected(Result<bool, String>),
    Back,
    Hangup,
    Copy,
    Paste,
    CharacterReceived(char),
    KeyPressed(KeyCode, Modifiers),
    KeyReleased(KeyCode, Modifiers),
    WheelScrolled(ScrollDelta),
    AltKeyPressed(bool),
    // FontSelected(String),
    // ScreenModeSelected(ScreenMode),
    SelectProtocol(ProtocolType, bool),
    OpenURL(String),
    CancelTransfer,
    
    SetSelection(Option<Selection>),

    ListAction(HoverListMessage),
    CreateNewBBS,

    // Phonebook
    QuickConnectChanged(String),

    // Edit BBS
    EditBbsSystemNameChanged(String),
    EditBbsAddressChanged(String),
    EditBbsUserNameChanged(String),
    EditBbsPasswordChanged(String),
    EditBbsCommentChanged(String),
    EditBbsTerminalTypeSelected(Terminal),
    EditBbsScreenModeSelected(ScreenMode),
    EditBbsAutoLoginChanged(String),
    EditBbsDeleteEntry,
    EditBbsConnectionType(ConnectionType),
}
