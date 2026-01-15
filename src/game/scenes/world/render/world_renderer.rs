use glam::UVec2;

use crate::{
    engine::renderer::{Frame, Renderer},
    game::scenes::world::{
        render::{
            GeometryBuffer,
            box_pipeline::{self, BoxPipeline},
            ui_pipeline::UiPipeline,
        },
        sim_world::{SimWorld, ecs::Snapshots},
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
    box_pipeline: BoxPipeline,

    /// Render bounding boxes?
    pub render_bounding_boxes: bool,

    /// Bounding boxes extracted from the sim world.
    bounding_boxes: Vec<box_pipeline::gpu::Instance>,
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
        let box_pipeline = BoxPipeline::new(renderer, render_store);

        Self {
            terrain_pipeline,
            model_pipeline,
            ui_pipeline,
            box_pipeline,

            render_bounding_boxes: false,
            bounding_boxes: Vec::default(),
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
        self.ui_pipeline
            .extract(sim_world, render_store, render_world, viewport_size);

        self.bounding_boxes.clear();
    }

    pub fn prepare(
        &mut self,
        renderer: &Renderer,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        snapshots: &Snapshots,
    ) {
        self.terrain_pipeline
            .prepare(renderer, render_world, &snapshots.terrain_render_snapshot);
        self.model_pipeline.prepare(
            renderer,
            render_store,
            render_world,
            &snapshots.model_render_snapshot,
        );
        self.ui_pipeline.prepare(renderer, render_world);
        self.box_pipeline
            .prepare(renderer, &snapshots.box_render_snapshot);
    }

    pub fn queue(
        &mut self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        snapshots: &Snapshots,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
    ) {
        self.terrain_pipeline.queue(
            render_world,
            frame,
            geometry_buffer,
            &snapshots.terrain_render_snapshot,
        );
        self.model_pipeline
            .queue(render_store, render_world, frame, geometry_buffer);
        self.ui_pipeline.queue(render_world, frame);

        if !self.bounding_boxes.is_empty() {
            self.box_pipeline
                .queue(frame, geometry_buffer, render_world);
        }
    }
}
