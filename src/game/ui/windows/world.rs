use std::sync::Arc;

use glam::{IVec2, UVec2};

use crate::{
    engine::{assets::AssetError, renderer::Gpu},
    game::{
        assets::models::Models,
        file_system::FileSystem,
        render::{
            geometry_buffer::GeometryBuffer,
            textures::Textures,
            world::{WorldRenderSnapshot, WorldRenderer},
        },
        scenes::world::sim_world::Terrain,
        ui::{
            Rect,
            render::window_renderer::{WindowRenderItems, WindowRenderer},
            windows::window::Window,
        },
    },
};

pub struct WorldWindow {
    gpu: Gpu,
    rect: Rect,

    geometry_buffer: GeometryBuffer,

    world_renderer: WorldRenderer,
}

impl WorldWindow {
    pub fn new(
        gpu: Gpu,
        file_system: Arc<FileSystem>,
        textures: Arc<Textures>,
        size: UVec2,
        terrain: &Terrain,
    ) -> Result<Self, AssetError> {
        let geometry_buffer = GeometryBuffer::new(gpu.clone(), size);

        let images = textures.images();
        let models = Arc::new(Models::new(file_system, images)?);

        let world_renderer = WorldRenderer::new(models, textures, gpu.clone(), terrain);

        Ok(Self {
            gpu,
            rect: Rect::new(IVec2::ZERO, IVec2::ZERO),
            geometry_buffer,
            world_renderer,
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

        let snapshot = WorldRenderSnapshot::default();

        self.world_renderer.prepare(&self.gpu, &snapshot);
        // TODO: self.world_renderer.queue(&self.geometry_buffer, &snapshot);
    }
}
