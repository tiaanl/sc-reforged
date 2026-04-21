use glam::{IVec2, Vec4};

use crate::game::ui::{
    EventResult, Rect,
    render::window_renderer::{Font, WindowRenderItems, WindowRenderer},
    u32_to_color,
};

use super::widget::Widget;

pub enum ListBoxItemKind {
    Text {
        font: Font,
        text: String,
        color: Option<Vec4>,
    },
}

pub struct ListBoxItem {
    height: i32,
    kind: ListBoxItemKind,
}

impl ListBoxItem {
    pub fn text(text: impl Into<String>, font: Font, color: Option<Vec4>) -> Self {
        let text = text.into();

        Self {
            height: 15,
            kind: ListBoxItemKind::Text { font, text, color },
        }
    }

    fn desired_size(&self, window_renderer: &WindowRenderer) -> IVec2 {
        match self.kind {
            ListBoxItemKind::Text { font, ref text, .. } => IVec2::new(
                window_renderer.measure_text_width(text.as_bytes(), font),
                self.height,
            ),
        }
    }

    fn render(&self, rect: Rect, window_render_items: &mut WindowRenderItems) {
        match self.kind {
            ListBoxItemKind::Text {
                font,
                ref text,
                color,
            } => window_render_items.render_text(rect.position, text.as_bytes(), font, color),
        }
    }
}

#[derive(Default)]
pub struct ListBoxWidget {
    rect: Rect,

    items: Vec<ListBoxItem>,

    scroll_offset: IVec2,
    pending_scroll_delta: i32,
    scroll_step_accumulator: i32,

    /// Calculated during a layout/render pass.
    content_size: i32,

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

    pub fn add_item(&mut self, item: ListBoxItem) {
        self.items.push(item);
    }
}

impl Widget for ListBoxWidget {
    fn rect(&self) -> Rect {
        self.rect
    }

    fn on_primary_mouse_down(&mut self, _mouse_position: IVec2) -> EventResult {
        /*
        int item_top = 0;
        int local_y = (mouse_y - widget.rect.top) + m_scroll_offset_y;

        if (m_input_disabled) {
            return EVENT_HANDLED;
        }

        // Do not allow this list box to begin a new drag while another drag is active.
        if (g_window_manager.drag_items.count != 0) {
            return EVENT_HANDLED;
        }

        // Iterate items from the visual start of the list.
        m_item_iter = m_item_tail;
        if (m_item_tail == NULL) {
            return EVENT_HANDLED;
        }

        while (m_item_iter != NULL) {
            List_Box_Item *item = m_item_iter->item;

            // item->height is the field Ghidra still shows as piVar8[3]
            if (item_top < local_y && local_y <= item_top + item->height) {
                break;
            }

            item_top += item->height;
            m_item_iter = m_item_iter->prev;
        }

        if (m_item_iter == NULL) {
            return EVENT_HANDLED;
        }

        List_Box_Item *clicked_item = m_item_iter->item;

        // Update selection if the clicked item is different.
        if (clicked_item != m_selected_item) {
            if (m_selected_item != NULL) {
                m_selected_item->is_selected = false;
            }

            m_selected_item = clicked_item;
            clicked_item->is_selected = true;

            if (m_selection_changed_callback != NULL) {
                m_selection_changed_callback(m_selection_changed_context);
            }
        }

        // Fire the generic "item pressed" callback even if selection did not change.
        if (m_item_pressed_callback != NULL) {
            m_item_pressed_callback(m_item_pressed_context);
        }

        // If the selected item supports dragging, begin a drag operation.
        if (m_selected_item != NULL &&
            m_selected_item->can_begin_drag &&
            g_window_manager.drag_items.count == 0)
        {
            m_selected_item->drag_reset_on_start = false;

            g_window_manager.drag_items.push_back(m_selected_item);

            g_window_manager.m_drag_width =
                widget.rect.right - widget.rect.left;

            g_window_manager.m_drag_offset_x =
                mouse_x - widget.rect.left;

            g_window_manager.m_drag_offset_y =
                local_y - item_top;
        }

        // Optionally dispatch the clicked item's own press handler.
        if (clicked_item != NULL && m_dispatch_item_press_handler) {
            clicked_item->On_Primary_Mouse_Down();
        }

        return EVENT_HANDLED;
        */

        todo!()
    }

    fn on_primary_mouse_up(&mut self, _mouse_position: IVec2) -> EventResult {
        /*
        if (m_selected_item != NULL && !m_input_disabled) {
            m_selected_item->On_Primary_Mouse_Up();
        }

        return EVENT_HANDLED;
        */

        todo!()
    }

    fn on_mouse_wheel(&mut self, wheel_steps: i32) -> EventResult {
        if self.mouse_wheel_disabled || self.items.is_empty() {
            return EventResult::Ignore;
        }

        let item_height = self.items.first().map(|item| item.height).unwrap_or(0);

        let mut target_scroll =
            self.scroll_offset.y + self.pending_scroll_delta + item_height * wheel_steps;

        let max_scroll = (self.content_size + self.rect.size.y) - self.rect.bottom_right().y;

        if target_scroll > max_scroll {
            target_scroll = max_scroll;
        }

        if target_scroll < 0 {
            target_scroll = 0;
        }

        if self.snap_scroll_to_item_extent {
            self.scroll_offset.y = target_scroll;
            // self.sync_scrollbar_position();
        } else {
            self.pending_scroll_delta = target_scroll - self.scroll_offset.y;
        }

        EventResult::Handled
    }

    fn render(
        &mut self,
        origin: IVec2,
        delta_time_ms: i32,
        window_renderer: &WindowRenderer,
        window_render_items: &mut WindowRenderItems,
    ) {
        let rect = self.rect.offset(origin);

        // 1. Draw list-box chrome.
        if self.render_panel_background && !self.force_flat_background {
            // Render_Panel_Pressed(x, y, width, height, g_interface_textures.m_window_motif_texture, 3, 0);
            // window_render_items.render_panel_pressed(rect);
        } else if self.draw_border {
            window_render_items.render_border(
                rect.offset(IVec2::NEG_ONE),
                1,
                u32_to_color(0xff263f99),
            );
        }

        // 2. Enable clipping for normal scrolling mode.
        if !self.snap_scroll_to_item_extent {
            window_render_items.push_clip_rect(rect);
        }

        // 3. Smoothly consume any pending scroll delta.
        if self.pending_scroll_delta != 0 {
            let accumulator = self.scroll_step_accumulator + delta_time_ms;
            let mut step = accumulator / 8;
            self.scroll_step_accumulator = accumulator - step * 8;

            if self.pending_scroll_delta < 0 {
                step = -step;
                if step < self.pending_scroll_delta {
                    step = self.pending_scroll_delta;
                }
            } else if self.pending_scroll_delta < step {
                step = self.pending_scroll_delta;
            }

            self.scroll_offset.y += step;
            self.pending_scroll_delta -= step;
            // self.sync_scrollbar_position();
        }

        // 4. Walk the items and render the ones that are visible.
        let mut item_offset = IVec2::ZERO;
        let scroll_offset = self.scroll_offset;
        let viewport_size = rect.size;

        self.content_size = 0;

        for item in self.items.iter() {
            let item_size = item.desired_size(window_renderer);

            let stop_now;
            let item_is_visible;

            if !self.snap_scroll_to_item_extent {
                stop_now = item_offset.x > scroll_offset.x + viewport_size.x
                    || item_offset.y > scroll_offset.y + viewport_size.y;

                item_is_visible = scroll_offset.x <= item_offset.x + item_size.x
                    && scroll_offset.y <= item_offset.y + item_size.y;
            } else {
                stop_now = item_offset.x + item_size.x > scroll_offset.x + viewport_size.x
                    || item_offset.y + item_size.y > scroll_offset.y + viewport_size.y;

                item_is_visible =
                    scroll_offset.x <= item_offset.x && scroll_offset.y <= item_offset.y;
            }

            if stop_now {
                break;
            }

            if item_is_visible {
                let draw_position = rect.position + item_offset - scroll_offset;
                item.render(Rect::new(draw_position, item_size), window_render_items);
            }

            if self.horizontal {
                item_offset.x += item_size.x;
                self.content_size = item_offset.x;
            } else {
                item_offset.y += item_size.y;
                self.content_size = item_offset.y;
            }
        }

        // 5. Restore renderer state if we enabled clipping.
        if !self.snap_scroll_to_item_extent {
            window_render_items.pop_clip_rect();
        }
    }
}
