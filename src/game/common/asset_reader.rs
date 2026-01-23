use crate::{
    engine::storage::{Handle, Storage},
    game::{image::Image, model::Model, scenes::world::animation::motion::Motion},
};

pub struct AssetReader {
    images: Storage<Image>,
    models: Storage<Model>,
    _motions: Storage<Motion>,
}

impl AssetReader {
    pub fn new(images: Storage<Image>, models: Storage<Model>, motions: Storage<Motion>) -> Self {
        Self {
            images,
            models,
            _motions: motions,
        }
    }

    #[inline]
    pub fn get_image(&self, handle: Handle<Image>) -> Option<&Image> {
        self.images.get(handle)
    }

    #[inline]
    pub fn get_model(&self, handle: Handle<Model>) -> Option<&Model> {
        self.models.get(handle)
    }

    #[inline]
    pub fn get_motion(&self, handle: Handle<Motion>) -> Option<&Motion> {
        self._motions.get(handle)
    }
}
