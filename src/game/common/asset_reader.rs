use crate::{
    engine::storage::{Handle, Storage},
    game::image::Image,
};

pub struct AssetReader {
    images: Storage<Image>,
}

impl AssetReader {
    pub fn new(images: Storage<Image>) -> Self {
        Self { images }
    }

    #[inline]
    pub fn get_image(&self, handle: Handle<Image>) -> Option<&Image> {
        self.images.get(handle)
    }
}
