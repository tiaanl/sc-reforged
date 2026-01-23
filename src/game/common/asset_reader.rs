use crate::{
    engine::storage::{Handle, Storage},
    game::{image::Image, model::Model},
};

pub struct AssetReader {
    images: Storage<Image>,
    models: Storage<Model>,
}

impl AssetReader {
    pub fn new(images: Storage<Image>, models: Storage<Model>) -> Self {
        Self { images, models }
    }

    #[inline]
    pub fn get_image(&self, handle: Handle<Image>) -> Option<&Image> {
        self.images.get(handle)
    }

    #[inline]
    pub fn get_model(&self, handle: Handle<Model>) -> Option<&Model> {
        self.models.get(handle)
    }
}
