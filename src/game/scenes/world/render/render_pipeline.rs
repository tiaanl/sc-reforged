use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        AssetReader,
        scenes::world::render::{GeometryBuffer, RenderStore, RenderWorld},
    },
};

pub trait RenderPipeline {
    /// The data passed to the [RenderPipeline] to use for rendering.
    type Snapshot;

    /// Prepare GPU resources that will be used when queueing commands to the GPU.
    fn prepare(
        &mut self,
        assets: &AssetReader,
        renderer: &Renderer,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        snapshot: &Self::Snapshot,
    );

    /// Queue draw commands to the GPU.
    fn queue(
        &self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &Self::Snapshot,
    );
}
