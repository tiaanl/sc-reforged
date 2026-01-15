use crate::{
    engine::renderer::{Frame, Renderer},
    game::scenes::world::render::{GeometryBuffer, RenderStore, RenderWorld},
};

pub trait Pipeline {
    type Snapshot;

    fn prepare(
        &mut self,
        renderer: &Renderer,
        render_store: &mut RenderStore,
        render_world: &mut RenderWorld,
        snapshot: &Self::Snapshot,
    );

    fn queue(
        &self,
        render_store: &RenderStore,
        render_world: &RenderWorld,
        frame: &mut Frame,
        geometry_buffer: &GeometryBuffer,
        snapshot: &Self::Snapshot,
    );
}
