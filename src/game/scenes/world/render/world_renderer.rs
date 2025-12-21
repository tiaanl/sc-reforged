use glam::UVec2;

use crate::{
    engine::renderer::{Frame, Renderer},
    game::scenes::world::{
        render::{GeometryBuffer, ui_pipeline::UiPipeline},
        sim_world::SimWorld,
    },
};

use super::{
    model_pipeline::ModelPipeline, render_store::RenderStore, render_world::RenderWorld,
    terrain_pipeline::TerrainPipeline,
};

pub struct WorldRenderer {
    // TODO: should not be pub.
    pub terrain_pipeline: TerrainPipeline,
    // TODO: should not be pub.
    pub model_pipeline: ModelPipeline,
    ui_pipeline: UiPipeline,
}

impl WorldRenderer {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        render_store: &RenderStore,
        sim_world: &SimWorld,
    ) -> Self {
        let terrain_pipeline = TerrainPipeline::new(renderer, render_store, sim_world);
        let model_pipeline = ModelPipeline::new(renderer, render_store);
        let ui_pipeline = UiPipeline::new(renderer, surface_format, render_store);

        Self {
            terrain_pipeline,
            model_pipeline,
            ui_pipeline,
        }
    }

    // TODO: should not pass render_world or render_store here.
    pub fn extract(
        &mut self,
        sim_world: &mut SimWorld,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        viewport_size: UVec2,
    ) {
        self.terrain_pipeline.extract(sim_world, render_world);
        self.model_pipeline.extract(sim_world, render_store);
        self.ui_pipeline
            .extract(sim_world, render_store, render_world, viewport_size);
    }

    pub fn prepare(&mut self, renderer: &Renderer, render_world: &mut RenderWorld) {
        self.terrain_pipeline.prepare(renderer, render_world);
        self.model_pipeline.prepare(renderer, render_world);
        self.ui_pipeline.prepare(renderer, render_world);
    }

    pub fn queue(
        &mut self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
    ) {
        self.terrain_pipeline
            .queue(render_world, frame, geometry_buffer);
        self.model_pipeline
            .queue(render_store, render_world, frame, geometry_buffer);
        self.ui_pipeline.queue(render_world, frame);
    }
}
