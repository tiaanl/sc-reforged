use std::sync::Arc;

use crate::{
    engine::{
        renderer::{Gpu, RenderContext, RenderTarget},
        shader_cache::ShaderCache,
    },
    game::{
        assets::models::Models,
        render::{
            geometry_buffer::GeometryBuffer,
            textures::Textures,
            world::{
                WorldRenderSnapshot,
                camera_render_pipeline::CameraRenderPipeline,
                gizmo_render_pipeline::GizmoRenderPipeline,
                model_render_pipeline::ModelRenderPipeline,
                render_bindings::RenderBindings,
                render_pipeline::{RenderPipeline, RenderPipelineList},
                terrain_render_pipeline::TerrainRenderPipeline,
            },
        },
        scenes::world::sim_world::Terrain,
    },
};

use super::render_layouts::RenderLayouts;

pub struct WorldRenderer {
    pipelines: RenderPipelineList,
    bindings: RenderBindings,
}

impl WorldRenderer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(models: Arc<Models>, textures: Arc<Textures>, gpu: Gpu, terrain: &Terrain) -> Self {
        // Warm up the shader cache.
        let mut shader_cache = ShaderCache::default();
        shader_cache.preload_all(&gpu.device);

        let mut layouts = RenderLayouts::new(gpu.clone());
        let bindings = RenderBindings::new(&gpu, &mut layouts);

        let mut pipelines = RenderPipelineList::default();

        pipelines.push(CameraRenderPipeline);

        pipelines.push(TerrainRenderPipeline::new(
            textures.images().as_ref(),
            &gpu,
            &mut layouts,
            &mut shader_cache,
            terrain,
        ));

        pipelines.push(ModelRenderPipeline::new(
            &gpu,
            &mut layouts,
            &mut shader_cache,
            Arc::clone(&textures),
            Arc::clone(&models),
        ));
        pipelines.push(GizmoRenderPipeline::new(
            &gpu,
            &mut layouts,
            &mut shader_cache,
        ));

        Self {
            pipelines,
            bindings,
        }
    }
}

impl WorldRenderer {
    pub fn prepare(&mut self, gpu: &Gpu, snapshot: &WorldRenderSnapshot) {
        self.pipelines.prepare(gpu, &mut self.bindings, snapshot);
    }

    pub fn queue(
        &self,
        render_context: &mut RenderContext,
        render_target: &RenderTarget,
        geometry_buffer: &GeometryBuffer,
        snapshot: &WorldRenderSnapshot,
    ) {
        self.pipelines.queue(
            &self.bindings,
            render_context,
            render_target,
            geometry_buffer,
            snapshot,
        );
    }
}
