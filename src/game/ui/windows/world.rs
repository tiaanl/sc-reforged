use std::sync::Arc;

use glam::{IVec2, UVec2};

use crate::{
    engine::{
        assets::AssetError,
        renderer::Gpu,
        storage::Handle,
    },
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
            render::window_renderer::{WindowRenderItems, WindowRenderer as UiWindowRenderer},
            windows::window::{Window, WindowRenderContext},
        },
    },
};

pub struct WorldWindow {
    rect: Rect,

    world_renderer: WorldRenderer,
    gbuffer: Handle<GeometryBuffer>,
}

impl WorldWindow {
    pub fn new(
        gpu: Gpu,
        file_system: Arc<FileSystem>,
        textures: Arc<Textures>,
        ui_window_renderer: &UiWindowRenderer,
        size: UVec2,
        terrain: &Terrain,
    ) -> Result<Self, AssetError> {
        let images = textures.images();
        let models = Arc::new(Models::new(file_system, images)?);

        let mut world_renderer = WorldRenderer::new(
            models,
            textures,
            gpu,
            ui_window_renderer.gbuffer_bind_group_layout(),
            terrain,
        );

        let gbuffer = world_renderer.register_gbuffer(size);

        Ok(Self {
            rect: Rect::new(IVec2::ZERO, IVec2::ZERO),
            world_renderer,
            gbuffer,
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

    fn render(
        &mut self,
        ctx: &mut WindowRenderContext<'_>,
        render_items: &mut WindowRenderItems,
    ) {
        let surface_size = ctx.window_renderer.surface_size();
        if self.world_renderer.gbuffer_size(self.gbuffer) != Some(surface_size) {
            tracing::info!(
                "Resizing world view gbuffer to {}x{}.",
                surface_size.x,
                surface_size.y,
            );
            self.world_renderer
                .resize_gbuffer(self.gbuffer, surface_size);
        }

        let snapshot = WorldRenderSnapshot::default();

        self.world_renderer.prepare(ctx.gpu, &snapshot);
        self.world_renderer
            .render_to(self.gbuffer, ctx.render_context, &snapshot);

        if let Some(bind_group) = self.world_renderer.gbuffer_bind_group(self.gbuffer) {
            render_items.render_world_view(bind_group);
        }
    }
}
