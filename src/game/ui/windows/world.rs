use glam::{IVec2, UVec2};

use crate::{
    engine::{assets::AssetError, renderer::Gpu},
    game::{
        render::geometry_buffer::GeometryBuffer,
        ui::{
            Rect,
            render::window_renderer::{WindowRenderItems, WindowRenderer},
            windows::window::Window,
        },
    },
};

pub struct WorldWindow {
    rect: Rect,

    geometry_buffer: GeometryBuffer,
}

impl WorldWindow {
    pub fn new(gpu: Gpu, size: UVec2) -> Result<Self, AssetError> {
        let geometry_buffer = GeometryBuffer::new(gpu, size);

        Ok(Self {
            rect: Rect::new(IVec2::ZERO, IVec2::ZERO),
            geometry_buffer,
        })
    }
}

impl Window for WorldWindow {
    fn is_visible(&self) -> bool {
        true
    }

    fn wants_input(&self) -> bool {
        true
    }

    fn hit_test(&self, position: glam::IVec2) -> bool {
        self.rect.contains(position)
    }

    fn rect(&self) -> crate::game::ui::Rect {
        todo!()
    }

    fn render(&mut self, window_renderer: &WindowRenderer, _render_items: &mut WindowRenderItems) {
        let surface_size = window_renderer.surface_size();
        if self.geometry_buffer.size != surface_size {
            tracing::info!(
                "Resizing geometry buffer for world window to {}x{}.",
                surface_size.x,
                surface_size.y
            );
            self.geometry_buffer.resize(surface_size);
        }
    }
}
