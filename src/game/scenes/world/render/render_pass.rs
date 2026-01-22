use crate::{
    engine::renderer::{Frame, Renderer},
    game::{
        assets::Assets,
        scenes::world::render::{GeometryBuffer, RenderStore, RenderWorld},
    },
};

pub trait RenderPass {
    /// The data passed to the [RenderPass] to use for rendering.
    type Snapshot;

    /// Prepare GPU resources that will be used when queueing commands to the GPU.
    fn prepare(
        &mut self,
        assets: &Assets,
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
