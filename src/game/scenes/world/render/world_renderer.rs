use std::sync::Arc;

use crate::{
    engine::{
        renderer::{Gpu, RenderContext, RenderTarget},
        shader_cache::ShaderCache,
    },
    game::{
        assets::{images::Images, models::Models},
        render::{compositor::Compositor, geometry_buffer::GeometryBuffer, textures::Textures},
        scenes::world::{
            extract::RenderSnapshot,
            render::{
                GizmoRenderPipeline,
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
        gpu: &Gpu,
        target_format: wgpu::TextureFormat,
        layouts: &mut RenderLayouts,
        shader_cache: &mut ShaderCache,
        sim_world: &bevy_ecs::world::World,
    ) -> Self {
        // Warm up the shader cache.
        shader_cache.preload_all(&gpu.device);

        let textures = Arc::new(Textures::new(gpu.clone(), Arc::clone(&images)));

        let mut pipelines = RenderPipelineList::default();

        pipelines.push(CameraRenderPipeline);
        pipelines.push(TerrainRenderPipeline::new(
            &images,
            gpu,
            layouts,
            shader_cache,
            sim_world,
        ));

        pipelines.push(ModelRenderPipeline::new(
            gpu,
            layouts,
            shader_cache,
            Arc::clone(&textures),
            Arc::clone(&models),
        ));
        pipelines.push(GizmoRenderPipeline::new(
            gpu,
            target_format,
            layouts,
            shader_cache,
        ));

        Self { pipelines }
    }
}

impl RenderPipeline for WorldRenderer {
    fn prepare(&mut self, gpu: &Gpu, bindings: &mut RenderBindings, snapshot: &RenderSnapshot) {
        self.pipelines.prepare(gpu, bindings, snapshot);
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    ) {
        self.pipelines.queue(
            bindings,
            render_context,
            render_target,
            geometry_buffer,
            snapshot,
        );
    }
}
