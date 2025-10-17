use ahash::HashMap;

use crate::{
    engine::{assets::AssetError, prelude::Renderer, storage::Handle},
    game::{model::Model, scenes::world::render::render_world::RenderWorld},
};

use super::{
    render_models::{RenderModel, RenderModels},
    render_textures::RenderTextures,
};

pub struct RenderStore {
    pub camera_bind_group_layout: wgpu::BindGroupLayout,

    pub models: RenderModels,
    pub textures: RenderTextures,

    /// Cache of model handles to render model handles.
    model_to_render_model: HashMap<Handle<Model>, Handle<RenderModel>>,
}

impl RenderStore {
    pub fn new(renderer: &Renderer) -> Self {
        let camera_bind_group_layout = RenderWorld::create_camera_bind_group_layout(renderer);

        let models = RenderModels::new();
        let textures = RenderTextures::new();

        let model_to_render_model = HashMap::default();

        Self {
            camera_bind_group_layout,

            models,
            textures,

            model_to_render_model,
        }
    }

    #[inline]
    pub fn get_or_create_render_model(
        &mut self,
        model_handle: Handle<Model>,
    ) -> Result<Handle<RenderModel>, AssetError> {
        if let Some(render_model_handle) = self.model_to_render_model.get(&model_handle) {
            return Ok(*render_model_handle);
        }

        let render_model_handle = self.models.add(&mut self.textures, model_handle)?;

        // Cache the new handle.
        self.model_to_render_model
            .insert(model_handle, render_model_handle);

        Ok(render_model_handle)
    }

    #[inline]
    pub fn render_model_for_model(&self, model: Handle<Model>) -> Option<Handle<RenderModel>> {
        self.model_to_render_model.get(&model).cloned()
    }
}
