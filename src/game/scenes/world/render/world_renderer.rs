use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                Compositor, GeometryBuffer, GizmoRenderPipeline, RenderTargets,
                camera_render_pipeline::CameraRenderPipeline, render_pipeline::RenderPipeline,
                ui_render_pipeline::UiRenderPipeline,
            },
        },
    },
};

use super::{
    model_render_pipeline::ModelRenderPipeline, render_layouts::RenderLayouts,
    render_world::RenderWorld, terrain_render_pipeline::TerrainRenderPipeline,
};

pub struct WorldRenderer {
    camera_pipeline: CameraRenderPipeline,
    // TODO: should not be pub.
    pub terrain_pipeline: TerrainRenderPipeline,
    // TODO: should not be pub.
    pub model_pipeline: ModelRenderPipeline,
    ui_pipeline: UiRenderPipeline,
    compositor: Compositor,
    gizmo_pipeline: GizmoRenderPipeline,
}

impl WorldRenderer {
    pub fn new(
        assets: &AssetReader,
        renderer: &Renderer,
        render_targets: &RenderTargets,
        layouts: &mut RenderLayouts,
        sim_world: &bevy_ecs::world::World,
    ) -> Self {
        let camera_pipeline = CameraRenderPipeline;
        let terrain_pipeline = TerrainRenderPipeline::new(assets, renderer, layouts, sim_world);
        let model_pipeline = ModelRenderPipeline::new(renderer, layouts);
        let ui_pipeline = UiRenderPipeline::new(renderer, render_targets.surface_format, layouts);
        let compositor = Compositor::new(renderer, render_targets);
        let gizmo_pipeline =
            GizmoRenderPipeline::new(renderer, render_targets.surface_format, layouts);

        Self {
            camera_pipeline,
            terrain_pipeline,
            model_pipeline,
            ui_pipeline,
            compositor,
            gizmo_pipeline,
        }
    }
}

impl RenderPipeline for WorldRenderer {
    fn prepare(
        &mut self,
        assets: &AssetReader,
        renderer: &Renderer,
        render_world: &mut RenderWorld,
        snapshot: &RenderSnapshot,
    ) {
        self.camera_pipeline
            .prepare(assets, renderer, render_world, snapshot);
        self.terrain_pipeline
            .prepare(assets, renderer, render_world, snapshot);
        self.model_pipeline
            .prepare(assets, renderer, render_world, snapshot);
        self.ui_pipeline
            .prepare(assets, renderer, render_world, snapshot);
        self.compositor
            .prepare(assets, renderer, render_world, snapshot);
        self.gizmo_pipeline
            .prepare(assets, renderer, render_world, snapshot);
    }

    fn queue(
        &self,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    ) {
        self.camera_pipeline
            .queue(render_world, frame, geometry_buffer, snapshot);
        self.terrain_pipeline
            .queue(render_world, frame, geometry_buffer, snapshot);
        self.model_pipeline
            .queue(render_world, frame, geometry_buffer, snapshot);
        self.ui_pipeline
            .queue(render_world, frame, geometry_buffer, snapshot);
        self.compositor
            .queue(render_world, frame, geometry_buffer, snapshot);
        self.gizmo_pipeline
            .queue(render_world, frame, geometry_buffer, snapshot);
    }
}
