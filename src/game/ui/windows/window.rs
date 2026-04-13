use crate::game::ui::render::window_renderer::{WindowRenderItems, WindowRenderer};

pub trait Window {
    /// Called to update the window state given the time in seconds since the last frame was drawn
    /// in `delta_time`.
    fn update(&mut self, delta_time: f32) {
        let _ = delta_time;
    }

    /// Called for each window so they can append items to the `render_items` to be rendered.
    fn render(&mut self, window_renderer: &WindowRenderer, render_items: &mut WindowRenderItems);
}
