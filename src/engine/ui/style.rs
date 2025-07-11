use crate::engine::ui::geometry::Color;

#[derive(Clone, Copy, Debug)]
pub enum Length {
    Auto,
    Pixels(i32),
}

#[derive(Clone, Copy, Debug)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

#[derive(Clone, Copy, Debug)]
pub struct Margin {
    inline_start: i32,
    inline_end: i32,
    block_start: i32,
    block_end: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    // Size
    pub width: Length,
    pub height: Length,

    // Alignment
    pub horizontal_align: Align,
    pub vertical_align: Align,

    pub margin: Margin,

    // Visual
    pub background_color: Color,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: Length::Auto,
            height: Length::Auto,

            horizontal_align: Align::Start,
            vertical_align: Align::Start,

            margin: Margin {
                inline_start: 0,
                inline_end: 0,
                block_start: 0,
                block_end: 0,
            },

            background_color: Color::WHITE,
        }
    }
}
