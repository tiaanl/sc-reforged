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
                camera_render_pipeline::CameraRenderPipeline,
                render_pipeline::{RenderPipeline, RenderPipelineList},
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
    pipelines: RenderPipelineList,
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

        let mut pipelines = RenderPipelineList::default();

        pipelines.push(CameraRenderPipeline);
        pipelines.push(TerrainRenderPipeline::new(
            assets,
            renderer,
            layouts,
            shader_cache,
            sim_world,
        ));

        pipelines.push(ModelRenderPipeline::new(renderer, layouts, shader_cache));
        pipelines.push(UiRenderPipeline::new(
            renderer,
            render_targets.surface_format,
            layouts,
            shader_cache,
        ));
        pipelines.push(Compositor::new(renderer, render_targets, shader_cache));
        pipelines.push(GizmoRenderPipeline::new(
            renderer,
            render_targets.surface_format,
            layouts,
            shader_cache,
        ));

        Self { pipelines }
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
        self.pipelines.prepare(assets, renderer, bindings, snapshot);
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    ) {
        self.pipelines
            .queue(bindings, frame, geometry_buffer, snapshot);
    }
}
