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

#[derive(Default)]
pub struct ListBoxWidget {
    rect: Rect,

    items: Vec<Rc<RefCell<dyn ListBoxItem>>>,

    pub render_panel_background: bool,
    pub force_flat_background: bool,
    pub draw_border: bool,
    pub snap_scroll_to_item_extent: bool,
    pub input_disabled: bool,
    pub mouse_wheel_disabled: bool,
    pub horizontal: bool,
}

impl ListBoxWidget {
    pub fn horizontal(rect: Rect) -> Self {
        Self {
            rect,
            horizontal: true,
            ..Default::default()
        }
    }

    pub fn vertical(rect: Rect) -> Self {
        Self {
            rect,
            ..Default::default()
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

        // 1. Draw list-box chrome.
        if self.render_panel_background && !self.force_flat_background {
            // Render_Panel_Pressed(x, y, width, height, g_interface_textures.m_window_motif_texture, 3, 0);
            // window_render_items.render_panel_pressed(rect);
        } else if self.draw_border {
            window_render_items.render_border(
                rect.offset(IVec2::NEG_ONE).grow(IVec2::ONE * 2),
                1,
                Vec4::new(153.0 / 255.0, 63.0 / 255.0, 38.0 / 255.0, 1.0),
            );
        }

        // 2. Enable clipping for normal scrolling mode.
        if self.snap_scroll_to_item_extent {
            window_render_items.push_clip_rect(rect);
        }

        /*
        // 3. Smoothly consume any pending scroll delta.
        //
        // Scroll speed is frame-time based: roughly 1 pixel per 8 ms,
        // clamped so it never overshoots the remaining pending amount.
        if (m_pending_scroll_delta != 0) {
            int accumulator = m_scroll_step_accumulator + g_event_processor.m_frame_delta_ms;
            int step = signed_divide_by_8(accumulator);   // keeps sign correct
            m_scroll_step_accumulator = accumulator - step * 8;

            if (m_pending_scroll_delta < 0) {
                step = -step;
                if (step < m_pending_scroll_delta) {
                    step = m_pending_scroll_delta;
                }
            } else {
                if (m_pending_scroll_delta < step) {
                    step = m_pending_scroll_delta;
                }
            }

            m_scroll_offset_y += step;
            m_pending_scroll_delta -= step;
            Sync_Scrollbar_Position();
        }
        */

        /*
        // 4. Walk the items and render the ones that are visible.
        int item_x = 0;
        int item_y = 0;

        bool vertical = !m_is_horizontal;
        int advance_x_per_item = m_is_horizontal ? 1 : 0;
        int advance_y_per_item = vertical ? 1 : 0;

        for (node = m_items.m_tail; node != NULL; node = node->prev) {
            List_Box_Item *item = node->item;

            int scroll_x = m_scroll_offset_x;
            int scroll_y = m_scroll_offset_y;

            bool stop_now;
            bool item_is_visible;

            if (!m_snap_scroll_to_item_extent) {
                // Normal mode:
                // render if the item intersects the viewport at all.
                stop_now =
                    (item_x > scroll_x + width) ||
                    (item_y > scroll_y + height);

                item_is_visible =
                    (scroll_x <= item_x + item->width) &&
                    (scroll_y <= item_y + item->height);
            } else {
                // Snap-to-item mode:
                // only render if the whole item origin is inside the viewport,
                // and stop once the next full item would extend beyond it.
                stop_now =
                    (item_x + item->width > scroll_x + width) ||
                    (item_y + item->height > scroll_y + height);

                item_is_visible =
                    (scroll_x <= item_x) &&
                    (scroll_y <= item_y);
            }

            if (stop_now) {
                break;
            }

            if (item_is_visible) {
                int draw_x = x + item_x - scroll_x;
                int draw_y = y + item_y - scroll_y;

                if (!m_is_horizontal) {
                    item->Render_Vertical(draw_x, draw_y, width);
                } else {
                    item->Render_Horizontal(draw_x, draw_y, height);
                }
            }

            item_x += advance_x_per_item * item->width;
            item_y += advance_y_per_item * item->height;
        }
        */

        // 5. Restore renderer state if we enabled clipping.
        if !self.snap_scroll_to_item_extent {
            window_render_items.pop_clip_rect();
        }
    }
}
