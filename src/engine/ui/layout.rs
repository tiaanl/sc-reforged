use crate::engine::ui::{
    Pos, Rect, Size, Style,
    widget::{DynWidget, Widget},
};

pub struct LayoutContext {
    pub screen_size: Size,
}

pub fn layout_in_rect(min_size: Size, style: &Style, in_rect: Rect) -> Rect {
    Rect {
        pos: in_rect.pos,
        size: min_size,
    }
}

pub fn layout_stacked<'c>(
    children: impl Iterator<Item = &'c mut DynWidget>,
    constraint: Rect,
    context: &LayoutContext,
) {
    for child in children {
        let constraint = layout_in_rect(child.min_size(), child.style(), constraint);
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

        let constraint = layout_in_rect(
            min_size,
            child.style(),
            Rect {
                pos: Pos {
                    left,
                    top: constraint.pos.top,
                },
                size: min_size,
            },
        );

        child.layout(constraint, context);
        left += min_size.width;
    }
}
