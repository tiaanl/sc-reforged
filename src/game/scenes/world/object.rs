use glam::Vec3;
use tracing::info;

use crate::engine::{
    arena::{Arena, Handle},
    renderer::Renderer,
};

use super::{
    models::{Model, Models},
    textures::Textures,
};

/// Represents an object inside the game world.
pub struct Object {
    pub position: Vec3,
    pub rotation: Vec3,
    pub model_handle: Handle<Model>,
}

impl Object {
    pub fn new(position: Vec3, rotation: Vec3, model: Handle<Model>) -> Self {
        Self {
            position,
            rotation,
            model_handle: model,
        }
    }
}

pub struct Objects {
    pub models: Models,
    pub textures: Textures,
    pub objects: Vec<Object>,
}

impl Objects {
    pub fn new(renderer: &Renderer) -> Self {
        let textures = Textures::default();
        let models = Models::new(renderer);
        Self {
            models,
            textures,
            objects: vec![],
        }
    }

    pub fn spawn(&mut self, object: Object) {
        self.objects.push(object);
    }

    pub fn render(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::TextureView,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        let handles = self
            .objects
            .iter()
            .map(|object| object.model_handle.clone())
            .collect::<Vec<_>>();

        self.models.render_multiple(
            renderer,
            encoder,
            output,
            &self.textures,
            camera_bind_group,
            &handles,
        );

        // self.objects.iter().for_each(|object| {
        //     if let Some(model) = self.models.get(&object.model_handle) {
        //         self.models.render_model(
        //             renderer,
        //             encoder,
        //             output,
        //             &self.gpu_camera.bind_group,
        //             model,
        //             &self.textures,
        //         );
        //     } else {
        //         warn!("Model not found in arena!");
        //     }
        // });
    }
}
