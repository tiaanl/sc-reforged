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
        render_world: &mut RenderBindings,
        snapshot: &RenderSnapshot,
    );

    /// Queue draw commands to the GPU.
    fn queue(
        &self,
        render_world: &RenderBindings,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    );
}
