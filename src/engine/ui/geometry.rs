#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pos {
    pub left: i32,
    pub top: i32,
}

impl Pos {
    pub const ZERO: Pos = Self { left: 0, top: 0 };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub const ZERO: Size = Size::new(0, 0);

    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

impl std::ops::Mul<i32> for Size {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self::Output {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl std::ops::Div<i32> for Size {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self::Output {
            width: self.width / rhs,
            height: self.height / rhs,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub pos: Pos,
    pub size: Size,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub const RED: Self = Self::from_rgba(255, 0, 0, 255);
    pub const GREEN: Self = Self::from_rgba(0, 255, 0, 255);
    pub const BLUE: Self = Self::from_rgba(0, 0, 255, 255);
    pub const WHITE: Self = Self::from_rgba(255, 255, 255, 255);

    pub const fn from_rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}
