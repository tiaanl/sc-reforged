use glam::IVec2;

pub struct SelectionRect {
    pub pos: IVec2,
    pub size: IVec2,
}

pub struct Ui {
    pub selection_rect: Option<SelectionRect>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            selection_rect: None,
        }
    }
}
