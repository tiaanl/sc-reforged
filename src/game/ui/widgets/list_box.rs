use std::{cell::RefCell, rc::Rc};

use glam::{IVec2, UVec2, Vec4};

use crate::game::ui::render::window_renderer::{Font, WindowRenderItems, WindowRenderer};

use super::widget::Widget;

pub trait ListBoxItem {
    fn desired_size(&self) -> UVec2;
    fn layout(&mut self, pos: IVec2, size: UVec2);
    fn render(&self, window_render_items: &mut WindowRenderItems);
}

pub struct TextListBoxItem {
    pub pos: IVec2,
    pub size: UVec2,
    pub font: Font,
    pub text: String,
}

impl TextListBoxItem {
    pub fn new(text: impl Into<String>, font: Font, window_renderer: &WindowRenderer) -> Self {
        let text = text.into();

        // TODO: For help-window body rows, match the original item behavior:
        // use the dedicated text list-box item styling, a fixed 15 px row
        // height, and the custom green text color instead of measuring the row
        // purely from the font metrics.
        let width = window_renderer.measure_text_width(text.as_bytes(), font);
        let height = window_renderer.measure_text_height(text.as_bytes(), font);

        Self {
            pos: IVec2::ZERO,
            size: UVec2::new(width, height),
            font,
            text,
        }
    }
}

impl ListBoxItem for TextListBoxItem {
    fn desired_size(&self) -> UVec2 {
        self.size
    }

    fn layout(&mut self, pos: IVec2, size: UVec2) {
        self.pos = pos;
        self.size = size;
    }

    fn render(&self, window_render_items: &mut WindowRenderItems) {
        // TODO: The quit-confirmation help body uses explicit custom text
        // coloring. Thread that through the item instead of relying on the
        // font default color here.
        window_render_items.render_text(self.pos, &self.text, self.font, None);
    }
}

pub struct ListBoxWidget {
    pos: IVec2,
    size: UVec2,

    items: Vec<Rc<RefCell<dyn ListBoxItem>>>,
    scroll_offset: IVec2,
    content_size: UVec2,

    render_panel_background: bool,
    draw_border: bool,
}

impl ListBoxWidget {
    pub fn new(pos: IVec2, size: UVec2) -> Self {
        Self {
            pos,
            size,

            items: Vec::default(),
            scroll_offset: IVec2::ZERO,
            content_size: UVec2::ZERO,

            render_panel_background: false,
            draw_border: false,
        }
    }

    pub fn add_item(&mut self, widget: Rc<RefCell<dyn ListBoxItem>>) {
        self.items.push(widget);
    }

    fn layout_items(&mut self, origin: IVec2) -> UVec2 {
        let mut current = origin;
        let mut content_size = UVec2::ZERO;

        // TODO: Lay items out in content space, then apply `scroll_offset`
        // during rendering. The original widget keeps content extents separate
        // from the visible viewport and scrolls/clips the item positions.
        for item in self.items.iter_mut() {
            let mut item_ref = item.borrow_mut();
            let item_size = item_ref.desired_size();
            item_ref.layout(current, item_size);

            current.y += item_size.y as i32;

            content_size.x = content_size.x.max(item_size.x);
            content_size.y += item_size.y;
        }

        content_size
    }

    fn render_items(&self, window_render_items: &mut WindowRenderItems) {
        // TODO: Cull or clip items against the list-box viewport before
        // rendering. Right now every row is drawn even when it should be
        // outside the help window body area.
        for item in self.items.iter() {
            item.borrow().render(window_render_items);
        }
    }
}

impl Widget for ListBoxWidget {
    fn render(
        &mut self,
        origin: IVec2,
        _window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    ) {
        let pos = origin + self.pos;
        let size = self.size;

        // TODO: Honor `render_panel_background` and `draw_border` like the
        // original list box instead of always drawing this debug border. The
        // help window body list should render with its panel background
        // disabled and without a hardcoded red outline.
        window_render_items.render_border(pos, size, 1, Vec4::new(1.0, 0.0, 0.0, 1.0));

        self.layout_items(pos);
        self.render_items(window_render_items);
    }
}
