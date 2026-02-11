use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{GeometryBuffer, RenderBindings},
        },
    },
};

pub trait RenderPipeline {
    /// Prepare GPU resources that will be used when queueing commands to the GPU.
    fn prepare(
        &mut self,
        assets: &AssetReader,
        renderer: &Renderer,
        bindings: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    );

    /// Queue draw commands to the GPU.
    fn queue(
        &self,
        bindings: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
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
    fn prepare(
        &mut self,
        assets: &AssetReader,
        renderer: &Renderer,
        bindings: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    ) {
        for pipeline in self.pipelines.iter_mut() {
            pipeline.prepare(assets, renderer, bindings, snapshot);
        }
    }

    fn queue(
        &self,
        bindings: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    ) {
        for pipeline in self.pipelines.iter() {
            pipeline.queue(bindings, frame, geometry_buffer, snapshot);
        }
    }
}
