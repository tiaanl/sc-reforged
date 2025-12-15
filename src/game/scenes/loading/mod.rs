use glam::UVec2;

use crate::engine::{
    assets::AssetError,
    context::EngineContext,
    input::InputState,
    renderer::{Frame, Renderer, Surface},
    scene::Scene,
};

pub trait SceneLoader {
    fn load_scene(
        self,
        renderer: &Renderer,
        surface: &Surface,
    ) -> Result<Box<dyn Scene>, AssetError>;
}

pub struct LoadingScene<L: SceneLoader> {
    engine_context: EngineContext,

    scene_loader: L,

    scene_sender: std::sync::mpsc::Sender<Box<dyn Scene>>,
    scene_receiver: std::sync::mpsc::Receiver<Box<dyn Scene>>,

    is_loading: bool,
}

impl<L: SceneLoader> LoadingScene<L> {
    pub fn new(engine_context: EngineContext, renderer: &Renderer, scene_loader: L) -> Self {
        let (scene_sender, scene_receiver) = std::sync::mpsc::channel();
        Self {
            engine_context,
            scene_loader,
            scene_sender,
            scene_receiver,
            is_loading: false,
        }
    }

    fn start_loading(&self) {
        std::thread::spawn(move || {
            println!("done!");
        });
    }
}

impl<L: SceneLoader> Scene for LoadingScene<L> {
    fn resize(&mut self, _size: UVec2) {}

    fn update(&mut self, _delta_time: f32, _input: &InputState) {
        if !self.is_loading {
            self.is_loading = true;
            self.start_loading();
        }
    }

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
