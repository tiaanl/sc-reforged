use glam::UVec2;

use crate::{
    engine::{
        renderer::{Gpu, RenderContext},
        shader_cache::ShaderCache,
        storage::{Handle, Storage},
    },
    game::{
        render::{
            geometry_buffer::GeometryBuffer,
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
        sim::Terrain,
    },
};

use super::render_layouts::RenderLayouts;

pub struct WorldRenderer {
    gpu: Gpu,
    gbuffer_layout: wgpu::BindGroupLayout,
    gbuffers: Storage<GeometryBuffer>,

    pipelines: RenderPipelineList,
    bindings: RenderBindings,
}

impl WorldRenderer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(gpu: Gpu, gbuffer_layout: &wgpu::BindGroupLayout, terrain: &Terrain) -> Self {
        // Warm up the shader cache.
        let mut shader_cache = ShaderCache::default();
        shader_cache.preload_all(&gpu.device);

        let mut layouts = RenderLayouts::new(gpu.clone());
        let bindings = RenderBindings::new(&gpu, &mut layouts);

        let mut pipelines = RenderPipelineList::default();

        pipelines.push(CameraRenderPipeline);

        pipelines.push(TerrainRenderPipeline::new(
            &gpu,
            &mut layouts,
            &mut shader_cache,
            terrain,
        ));

        pipelines.push(ModelRenderPipeline::new(
            &gpu,
            &mut layouts,
            &mut shader_cache,
        ));
        pipelines.push(GizmoRenderPipeline::new(
            &gpu,
            &mut layouts,
            &mut shader_cache,
        ));

        Self {
            gpu,
            gbuffer_layout: gbuffer_layout.clone(),
            gbuffers: Storage::default(),
            pipelines,
            bindings,
        }
    }

    /// Register a new gbuffer of the given size and return a handle to it.
    pub fn register_gbuffer(&mut self, size: UVec2) -> Handle<GeometryBuffer> {
        let gbuffer = GeometryBuffer::new(self.gpu.clone(), self.gbuffer_layout.clone(), size);
        self.gbuffers.insert(gbuffer)
    }

    /// Returns the current size of the gbuffer for the given handle.
    pub fn gbuffer_size(&self, handle: Handle<GeometryBuffer>) -> Option<UVec2> {
        self.gbuffers.get(handle).map(|gbuffer| gbuffer.size)
    }

    /// Resize the gbuffer behind the given handle.
    pub fn resize_gbuffer(&mut self, handle: Handle<GeometryBuffer>, size: UVec2) {
        if let Some(gbuffer) = self.gbuffers.get_mut(handle) {
            gbuffer.resize(size);
        }
    }

    /// Returns a clone of the gbuffer's bind group, suitable for embedding in a
    /// window render item that the compositor will sample.
    pub fn gbuffer_bind_group(&self, handle: Handle<GeometryBuffer>) -> Option<wgpu::BindGroup> {
        self.gbuffers
            .get(handle)
            .map(|gbuffer| gbuffer.bind_group().clone())
    }

    pub fn prepare(&mut self, gpu: &Gpu, snapshot: &WorldRenderSnapshot) {
        self.pipelines.prepare(gpu, &mut self.bindings, snapshot);
    }

    /// Clear the gbuffer behind `handle` and queue every gbuffer-writing
    /// pipeline into it. The compositor (owned by `WindowRenderer`) is
    /// responsible for moving the result onto a render target later.
    pub fn render_to(
        &self,
        handle: Handle<GeometryBuffer>,
        render_context: &mut RenderContext,
        snapshot: &WorldRenderSnapshot,
    ) {
        let Some(gbuffer) = self.gbuffers.get(handle) else {
            return;
        };

        gbuffer.clear(&mut render_context.encoder, snapshot.environment.fog_color);

        self.pipelines
            .queue(&self.bindings, render_context, gbuffer, snapshot);
    }
}
