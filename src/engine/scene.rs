use super::renderer::Renderer;
use bevy_ecs::prelude::*;

pub trait Scene {
    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, delta_time: f32);

    fn begin_frame(&mut self) {}

    fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
    );

    fn end_frame(&mut self) {}
}

#[derive(Resource)]
pub struct SceneResource(pub Box<dyn Scene>);

unsafe impl Sync for SceneResource {}
unsafe impl Send for SceneResource {}

impl std::ops::Deref for SceneResource {
    type Target = dyn Scene;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl std::ops::DerefMut for SceneResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
