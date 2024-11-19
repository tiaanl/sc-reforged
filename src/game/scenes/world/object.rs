use glam::Vec3;

use crate::engine::{arena::Handle, renderer::Renderer, shaders::Shaders};

use super::{
    bounding_boxes::BoundingBoxes,
    models::{Model, Models, RenderInfo},
    textures::Textures,
};

/// Represents an object inside the game world.
#[derive(Debug)]
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
    pub fn new(renderer: &Renderer, shaders: &mut Shaders) -> Self {
        let textures = Textures::default();
        let models = Models::new(renderer, shaders);
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
        bounding_boxes: &mut BoundingBoxes,
    ) {
        let handles = self
            .objects
            .iter()
            .map(|object| {
                RenderInfo::new(
                    object.position,
                    object.rotation,
                    object.model_handle.clone(),
                )
            })
            .collect::<Vec<_>>();

        if !handles.is_empty() {
            self.models.render_multiple(
                renderer,
                encoder,
                output,
                &self.textures,
                camera_bind_group,
                &handles,
                bounding_boxes,
            );
        }
    }
}
