use glam::UVec2;

use crate::{
    engine::{
        input::InputEvent,
        renderer::{RenderContext, RenderTarget},
        shader_cache::ShaderCache,
        storage::Handle,
    },
    game::{
        render::{compositor::Compositor, geometry_buffer::GeometryBuffer, world::WorldRenderer},
        sim::SimWorld,
    },
};

/// Native-resolution world background rendered behind the logical UI stack.
pub struct WorldLayer {
    sim: SimWorld,
    world_renderer: WorldRenderer,
    gbuffer: Handle<GeometryBuffer>,
    compositor: Compositor,
}

impl WorldLayer {
    /// Creates a world layer sized for the current surface.
    pub fn new(size: UVec2, target_format: wgpu::TextureFormat, mut sim: SimWorld) -> Self {
        let gbuffer_layout = GeometryBuffer::create_bind_group_layout();
        let mut shader_cache = ShaderCache::default();
        let compositor = Compositor::new(target_format, &gbuffer_layout, &mut shader_cache);

        let mut world_renderer = WorldRenderer::new(&gbuffer_layout, sim.terrain());
        let gbuffer = world_renderer.register_gbuffer(size);
        sim.resize_viewport(size);

        Self {
            sim,
            world_renderer,
            gbuffer,
            compositor,
        }
    }

    /// Resizes the world render targets and simulation viewport.
    pub fn resize(&mut self, size: UVec2) {
        if self.world_renderer.gbuffer_size(self.gbuffer) != Some(size) {
            tracing::info!("Resizing world layer gbuffer to {}x{}.", size.x, size.y);
            self.world_renderer.resize_gbuffer(self.gbuffer, size);
        }

        self.sim.resize_viewport(size);
    }

    /// Forwards an input event to the native-resolution simulation.
    pub fn input(&mut self, event: &InputEvent) {
        self.sim.input(event);
    }

    /// Advances the world simulation.
    pub fn update(&mut self, delta_time: f32) {
        self.sim.update(delta_time);
    }

    /// Renders the world to its gbuffer and composites it into the surface.
    pub fn render(&mut self, render_context: &mut RenderContext, render_target: &RenderTarget) {
        self.resize(render_target.size);

        let snapshot = self.sim.extract_snapshot();
        self.world_renderer.prepare(snapshot);
        self.world_renderer
            .render_to(self.gbuffer, render_context, snapshot);

        if let Some(bind_group) = self.world_renderer.gbuffer_bind_group(self.gbuffer) {
            self.compositor
                .composite(render_context, render_target, &bind_group);
        }
    }
}
