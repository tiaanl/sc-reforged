use std::sync::Arc;

use crate::{
    engine::{
        renderer::{Frame, RenderContext},
        shader_cache::ShaderCache,
    },
    game::{
        assets::{images::Images, models::Models},
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
        images: Arc<Images>,
        models: Arc<Models>,
        context: &RenderContext,
        render_targets: &RenderTargets,
        layouts: &mut RenderLayouts,
        shader_cache: &mut ShaderCache,
        sim_world: &bevy_ecs::world::World,
    ) -> Self {
        // Warm up the shader cache.
        shader_cache.preload_all(&context.device);

        let mut pipelines = RenderPipelineList::default();

        pipelines.push(CameraRenderPipeline);
        pipelines.push(TerrainRenderPipeline::new(
            &images,
            context,
            layouts,
            shader_cache,
            sim_world,
        ));

        pipelines.push(ModelRenderPipeline::new(
            context,
            layouts,
            shader_cache,
            Arc::clone(&images),
            Arc::clone(&models),
        ));
        pipelines.push(UiRenderPipeline::new(
            context,
            render_targets.surface_format,
            layouts,
            shader_cache,
        ));
        pipelines.push(Compositor::new(context, render_targets, shader_cache));
        pipelines.push(GizmoRenderPipeline::new(
            context,
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
        context: &RenderContext,
        bindings: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    ) {
        self.pipelines.prepare(context, bindings, snapshot);
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
