#[derive(Clone, Copy, PartialEq)]
pub struct Pos {
    pub left: u32,
    pub top: u32,
}

impl Pos {
    pub const ZERO: Pos = Self { left: 0, top: 0 };
}

#[derive(Clone, Copy, PartialEq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Rect {
    pub pos: Pos,
    pub size: Size,
}
