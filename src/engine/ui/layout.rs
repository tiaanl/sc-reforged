use crate::engine::ui::{
    Pos, Rect, Size, Style,
    widget::{DynWidget, Widget},
};

pub struct LayoutContext {
    pub screen_size: Size,
}

pub fn layout_in_rect(style: &Style, rect: &Rect) -> Rect {
    *rect
}

pub fn layout_stacked<'c>(
    children: impl Iterator<Item = &'c mut DynWidget>,
    constraint: Rect,
    context: &LayoutContext,
) {
    for child in children {
        child.layout(constraint, context);
    }
}

pub fn layout_horizontal<'c>(
    children: impl Iterator<Item = &'c mut DynWidget>,
    constraint: Rect,
    context: &LayoutContext,
) {
    let mut left = 0;
    for child in children {
        let min_size = child.min_size();
        child.layout(
            Rect {
                pos: Pos {
                    left,
                    top: constraint.pos.top,
                },
                size: min_size,
            },
            context,
        );
        left += min_size.width;
    }
}
