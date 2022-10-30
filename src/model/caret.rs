use super::{Position, TextAttribute};

pub struct Caret {
    pub(super) pos: Position,
    pub(super) attr: TextAttribute,
    pub insert_mode: bool
}

impl Caret {
    pub fn new() -> Self {
        Self {
            pos: Position::new(),
            attr: TextAttribute::DEFAULT,
            insert_mode: false
        }
    }
    
    pub fn get_foreground(self) -> u8
    {
        self.attr.get_foreground()
    }

    pub fn get_background(self) -> u8
    {
        self.attr.get_background()
    }

    pub fn get_attribute(&self) -> TextAttribute
    {
        self.attr
    }

    pub fn get_position(&self) -> Position
    {
        self.pos
    }

    pub(super) fn set_foreground(&mut self, color: u8) 
    {
        self.attr.set_foreground(color);
    }

    pub(super) fn set_background(&mut self, color: u8) 
    {
        self.attr.set_background(color);
    }
}

impl std::fmt::Debug for Caret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cursor").field("pos", &self.pos).field("attr", &self.attr).field("insert_mode", &self.insert_mode).finish()
    }
}

impl Default for Caret {
    fn default() -> Self {
        Self {
            pos: Position::default(),
            attr: TextAttribute::DEFAULT,
            insert_mode: Default::default()
        }
    }
}

impl PartialEq for Caret {
    fn eq(&self, other: &Caret) -> bool {
        self.pos == other.pos && self.attr == other.attr
    }
}
