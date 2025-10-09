use crate::{engine::prelude::Renderer, game::scenes::world::render_world::RenderWorld};

pub struct RenderStore {
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderStore {
    pub fn new(renderer: &Renderer) -> Self {
        let camera_bind_group_layout = RenderWorld::create_camera_bind_group_layout(renderer);
        Self {
            camera_bind_group_layout,
        }
    }
}
