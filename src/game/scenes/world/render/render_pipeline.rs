use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        AssetReader,
        scenes::world::{
            extract::RenderSnapshot,
            render::{GeometryBuffer, RenderStore, RenderWorld},
        },
    },
};

pub trait RenderPipeline {
    /// Prepare GPU resources that will be used when queueing commands to the GPU.
    fn prepare(
        &mut self,
        assets: &AssetReader,
        renderer: &Renderer,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        snapshot: &RenderSnapshot,
    );

    /// Queue draw commands to the GPU.
    fn queue(
        &self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &RenderSnapshot,
    );
}
