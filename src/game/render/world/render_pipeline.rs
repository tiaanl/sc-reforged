use crate::{
    engine::renderer::RenderContext,
    game::render::{
        geometry_buffer::GeometryBuffer,
        world::{render_bindings::RenderBindings, world_render_snapshot::WorldRenderSnapshot},
    },
};

pub trait RenderPipeline {
    /// Prepare GPU resources that will be used when queueing commands to the GPU.
    fn prepare(&mut self, bindings: &mut RenderBindings, snapshot: &WorldRenderSnapshot);

    /// Queue draw commands to the GPU.
    fn queue(
        &self,
        bindings: &RenderBindings,
        render_context: &mut RenderContext,
        geometry_buffer: &GeometryBuffer,
        snapshot: &WorldRenderSnapshot,
    );
}

#[derive(Default)]
pub struct RenderPipelineList {
    pipelines: Vec<Box<dyn RenderPipeline>>,
}

impl RenderPipelineList {
    pub fn push<T: 'static + RenderPipeline>(&mut self, pipeline: T) {
        self.pipelines.push(Box::new(pipeline));
    }
}

impl RenderPipeline for RenderPipelineList {
    fn prepare(&mut self, bindings: &mut RenderBindings, snapshot: &WorldRenderSnapshot) {
        for pipeline in self.pipelines.iter_mut() {
            pipeline.prepare(bindings, snapshot);
        }
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        render_context: &mut RenderContext,
        geometry_buffer: &GeometryBuffer,
        snapshot: &WorldRenderSnapshot,
    ) {
        for pipeline in self.pipelines.iter() {
            pipeline.queue(bindings, render_context, geometry_buffer, snapshot);
        }
    }
}
