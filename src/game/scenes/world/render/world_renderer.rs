use crate::{
    engine::renderer::{Frame, Renderer},
    game::scenes::world::{
        render::{
            GeometryBuffer, box_render_pass::BoxRenderPass, render_pass::RenderPass,
            ui_render_pass::UiRenderPass,
        },
        sim_world::{SimWorld, ecs::Snapshots},
    },
};

use super::{
    model_render_pass::ModelRenderPass, render_store::RenderStore, render_world::RenderWorld,
    terrain_render_pass::TerrainRenderPass,
};

pub struct WorldRenderer {
    // TODO: should not be pub.
    pub terrain_pipeline: TerrainRenderPass,
    // TODO: should not be pub.
    pub model_pipeline: ModelRenderPass,
    ui_pipeline: UiRenderPass,
    box_pipeline: BoxRenderPass,

    /// Render bounding boxes?
    pub render_bounding_boxes: bool,
}

impl WorldRenderer {
    pub fn new(
        renderer: &Renderer,
        surface_format: wgpu::TextureFormat,
        render_store: &mut RenderStore,
        sim_world: &SimWorld,
    ) -> Self {
        let terrain_pipeline = TerrainRenderPass::new(renderer, render_store, sim_world);
        let model_pipeline = ModelRenderPass::new(renderer, render_store);
        let ui_pipeline = UiRenderPass::new(renderer, surface_format, render_store);
        let box_pipeline = BoxRenderPass::new(renderer, render_store);

        Self {
            terrain_pipeline,
            model_pipeline,
            ui_pipeline,
            box_pipeline,

            render_bounding_boxes: false,
        }
    }

    pub fn prepare(
        &mut self,
        renderer: &Renderer,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        snapshots: &Snapshots,
    ) {
        self.terrain_pipeline.prepare(
            renderer,
            render_store,
            render_world,
            &snapshots.terrain_render_snapshot,
        );
        self.model_pipeline.prepare(
            renderer,
            render_store,
            render_world,
            &snapshots.model_render_snapshot,
        );
        self.ui_pipeline.prepare(
            renderer,
            render_store,
            render_world,
            &snapshots.ui_render_snapshot,
        );
        self.box_pipeline.prepare(
            renderer,
            render_store,
            render_world,
            &snapshots.box_render_snapshot,
        );
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
            render_store,
            render_world,
            frame,
            geometry_buffer,
            &snapshots.terrain_render_snapshot,
        );
        self.model_pipeline.queue(
            render_store,
            render_world,
            frame,
            geometry_buffer,
            &snapshots.model_render_snapshot,
        );
        self.ui_pipeline.queue(
            render_store,
            render_world,
            frame,
            geometry_buffer,
            &snapshots.ui_render_snapshot,
        );

        if self.render_bounding_boxes {
            self.box_pipeline.queue(
                render_store,
                render_world,
                frame,
                geometry_buffer,
                &snapshots.box_render_snapshot,
            );
        }
    }
}
