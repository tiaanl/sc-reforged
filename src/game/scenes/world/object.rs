use glam::Vec3;

use crate::{
    engine::{assets::Handle, renderer::Renderer, shaders::Shaders},
    game::models::{Model, ModelRenderer, RenderInfo},
};

/// Data only needed for a specific type of object.
#[derive(Debug)]
pub enum ObjectType {
    _4x4,
    Scenery,
    SceneryBush,
    SceneryLit,
    SceneryStripLight,
    Structure,
    StructureFence,
    StructureSwingDoor,
}

/// Represents an object inside the game world.
#[derive(Debug)]
pub struct Object {
    pub position: Vec3,
    pub rotation: Vec3,
    pub model_handle: Handle<Model>,
    pub _object_type: ObjectType,
}

impl Object {
    pub fn new(
        position: Vec3,
        rotation: Vec3,
        model_handle: Handle<Model>,
        object_type: ObjectType,
    ) -> Self {
        Self {
            position,
            rotation,
            model_handle,
            _object_type: object_type,
        }
    }
}

pub struct Objects {
    pub model_renderer: ModelRenderer,
    pub objects: Vec<Object>,
}

impl Objects {
    pub fn new(renderer: &Renderer, shaders: &mut Shaders) -> Self {
        let models = ModelRenderer::new(renderer, shaders);
        Self {
            model_renderer: models,
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
        // bounding_boxes: &mut BoundingBoxes,
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
            self.model_renderer.render_multiple(
                renderer,
                encoder,
                output,
                camera_bind_group,
                &handles,
                // bounding_boxes,
                wgpu::LoadOp::Load,
            );
        }
    }
}
