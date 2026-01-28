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
