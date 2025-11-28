use glam::{UVec2, Vec4};

pub struct UiRect {
    pub pos: UVec2,
    pub size: UVec2,
    pub color: Vec4,
}

pub struct Ui {
    pub ui_rects: Vec<UiRect>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            ui_rects: Vec::default(),
        }
    }
}
