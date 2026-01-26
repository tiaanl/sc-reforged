use crate::engine::renderer::Renderer;

use super::RenderWorld;

pub struct RenderLayouts {
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub ui_state_bind_group_layout: wgpu::BindGroupLayout,
}

impl RenderLayouts {
    pub fn new(renderer: &Renderer) -> Self {
        let camera_bind_group_layout = RenderWorld::create_camera_bind_group_layout(renderer);
        let ui_state_bind_group_layout = RenderWorld::create_ui_state_bind_group_layout(renderer);

        Self {
            camera_bind_group_layout,
            ui_state_bind_group_layout,
        }
    }
}
