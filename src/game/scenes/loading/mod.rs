use glam::UVec2;

use crate::engine::{
    input::InputState,
    renderer::{Frame, Renderer},
    scene::{LoadContext, Scene, SceneLoader},
};

pub struct LoadingScene<L: SceneLoader> {
    _phantom: std::marker::PhantomData<L>,
}

impl<L: SceneLoader> LoadingScene<L> {
    pub fn new(load_context: LoadContext, scene_loader: L) -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<L: SceneLoader> Scene for LoadingScene<L> {
    fn resize(&mut self, _size: UVec2) {}

    fn update(&mut self, _delta_time: f32, _input: &InputState) {}

    fn render(&mut self, _renderer: &Renderer, frame: &mut Frame) {
        let _render_pass = frame
            .encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("loading_scene_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.surface,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
    }
}
