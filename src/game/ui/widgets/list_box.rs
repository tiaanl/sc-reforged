use std::{cell::RefCell, rc::Rc};

use glam::{IVec2, Vec4};

use crate::game::ui::{
    Rect,
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
};

use super::widget::Widget;

pub trait ListBoxItem {
    fn desired_size(&self) -> IVec2;
    fn layout(&mut self, rect: Rect);
    fn render(&self, window_render_items: &mut WindowRenderItems);
}

pub struct TextListBoxItem {
    pub rect: Rect,
    pub font: Font,
    pub text: String,
}

impl TextListBoxItem {
    pub fn new(text: impl Into<String>, font: Font, window_renderer: &WindowRenderer) -> Self {
        let text = text.into();

        // TODO: For help-window body rows, match the original item behavior:
        // keep the measured text width, but use the dedicated list-box text row
        // styling with a fixed 15 px row height and explicit help-text color.
        let width = window_renderer.measure_text_width(text.as_bytes(), font);
        let height = window_renderer.measure_text_height(text.as_bytes(), font);

        Self {
            rect: Rect::new(IVec2::ZERO, IVec2::new(width, height)),
            font,
            text,
        }
    }
}

impl ListBoxItem for TextListBoxItem {
    fn desired_size(&self) -> IVec2 {
        self.rect.size
    }

    fn layout(&mut self, rect: Rect) {
        self.rect = rect;
    }

    fn render(&self, window_render_items: &mut WindowRenderItems) {
        // TODO: The quit-confirmation help body uses explicit custom green text
        // color (0xff19ff19). Store that on the item and pass it through here
        // instead of relying on the font default color.
        window_render_items.render_text(self.rect.position, self.text.as_bytes(), self.font, None);
    }
}

pub struct ListBoxWidget {
    rect: Rect,

    items: Vec<Rc<RefCell<dyn ListBoxItem>>>,

    render_panel_background: bool,
    draw_border: bool,
}

impl ListBoxWidget {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,

            items: Vec::default(),

            render_panel_background: false,
            draw_border: false,
        }
    }

    pub fn add_item(&mut self, widget: Rc<RefCell<dyn ListBoxItem>>) {
        self.items.push(widget);
    }

    fn layout_items(&mut self, origin: IVec2) -> IVec2 {
        let mut current = origin;
        let mut content_size = IVec2::ZERO;

        // TODO: This vertical stacking is enough for the quit-confirmation
        // help window. If another help window needs more body lines than fit in
        // the viewport, preserve this content size and reintroduce scrolling.
        for item in self.items.iter_mut() {
            let mut item_ref = item.borrow_mut();
            let item_size = item_ref.desired_size();
            item_ref.layout(Rect::new(current, item_size));

            current.y += item_size.y;

            content_size.x = content_size.x.max(item_size.x);
            content_size.y += item_size.y;
        }

        content_size
    }

    fn render_items(&self, window_render_items: &mut WindowRenderItems) {
        // TODO: Clipping keeps the quit-confirmation body correct already. Add
        // explicit culling later if longer help text makes this path expensive.
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
        let rect = self.rect.offset(origin);

        // TODO: Honor `render_panel_background` and `draw_border` like the
        // original list box instead of always drawing this debug border. For
        // the quit-confirmation help window, this list box should render with
        // no panel background and no extra frame of its own.
        window_render_items.render_border(rect, 1, Vec4::new(1.0, 0.0, 0.0, 1.0));

        self.layout_items(rect.position);
        window_render_items.push_clip_rect(rect);
        self.render_items(window_render_items);
        window_render_items.pop_clip_rect();
    }
}
