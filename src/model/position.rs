use std::{cmp::Ordering, ops::{Add, Sub}, hash::Hash};

#[derive(Copy, Clone, Debug, Eq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Hash for Position {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(x: {}, y: {})", self.x, self.y)
    }
}

impl Position {
    pub fn new() -> Self {
        Position { x: 0, y: 0 }
    }
    pub fn from(x: i32, y: i32) -> Self {
        Position { x, y }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

impl Add<Position> for Position {
    type Output = Position;

    fn add(self, rhs: Position) -> Position {
        Position { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl Sub<Position> for Position {
    type Output = Position;

    fn sub(self, rhs: Position) -> Position {
        Position { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Position) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.y < other.y { return Some(Ordering::Less); }
        if self.y > other.y { return Some(Ordering::Greater); }
        if self.x < other.x { return Some(Ordering::Less); }
        if self.x > other.x { return Some(Ordering::Greater); }
        Some(Ordering::Equal)
    }
}