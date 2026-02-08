use crate::{
    engine::{
        renderer::{Frame, Renderer},
        shader_cache::ShaderCache,
    },
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
    model_render_pipeline::ModelRenderPipeline, render_bindings::RenderBindings,
    render_layouts::RenderLayouts, terrain_render_pipeline::TerrainRenderPipeline,
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
        shader_cache: &mut ShaderCache,
        sim_world: &bevy_ecs::world::World,
    ) -> Self {
        // Warm up the shader cache.
        shader_cache.preload_all(&renderer.device);

        let camera_pipeline = CameraRenderPipeline;
        let terrain_pipeline =
            TerrainRenderPipeline::new(assets, renderer, layouts, shader_cache, sim_world);
        let model_pipeline = ModelRenderPipeline::new(renderer, layouts, shader_cache);
        let ui_pipeline = UiRenderPipeline::new(
            renderer,
            render_targets.surface_format,
            layouts,
            shader_cache,
        );
        let compositor = Compositor::new(renderer, render_targets, shader_cache);
        let gizmo_pipeline = GizmoRenderPipeline::new(
            renderer,
            render_targets.surface_format,
            layouts,
            shader_cache,
        );

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
        bindings: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    ) {
        self.camera_pipeline
            .prepare(assets, renderer, bindings, snapshot);
        self.terrain_pipeline
            .prepare(assets, renderer, bindings, snapshot);
        self.model_pipeline
            .prepare(assets, renderer, bindings, snapshot);
        self.ui_pipeline
            .prepare(assets, renderer, bindings, snapshot);
        self.compositor
            .prepare(assets, renderer, bindings, snapshot);
        self.gizmo_pipeline
            .prepare(assets, renderer, bindings, snapshot);
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    ) {
        self.camera_pipeline
            .queue(bindings, frame, geometry_buffer, snapshot);
        self.terrain_pipeline
            .queue(bindings, frame, geometry_buffer, snapshot);
        self.model_pipeline
            .queue(bindings, frame, geometry_buffer, snapshot);
        self.ui_pipeline
            .queue(bindings, frame, geometry_buffer, snapshot);
        self.compositor
            .queue(bindings, frame, geometry_buffer, snapshot);
        self.gizmo_pipeline
            .queue(bindings, frame, geometry_buffer, snapshot);
    }
}
