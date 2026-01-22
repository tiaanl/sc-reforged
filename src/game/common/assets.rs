use std::path::{Path, PathBuf};

use ahash::HashMap;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, Storage},
    },
    game::{
        config::{ImageDefs, parser::ConfigLines},
        file_system::file_system,
        image::Image,
    },
};

pub trait Asset: Sized + 'static {
    fn from_memory(
        context: &mut AssetLoadContext,
        path: PathBuf,
        data: &[u8],
    ) -> Result<Self, AssetError>;
}

/// Cache of loaded assets.
pub struct Assets {
    _image_defs: ImageDefs,

    images: AssetCache<Image>,
}

impl Assets {
    pub fn new() -> Result<Self, AssetError> {
        let _image_defs = load_config(PathBuf::from("config").join("image_defs.txt"))?;

        Ok(Self {
            _image_defs,
            images: Default::default(),
        })
    }

    #[inline]
    pub fn load_raw(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        Ok(file_system().load(path)?)
    }

    pub fn get_or_load_image(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(Handle<Image>, &Image), AssetError> {
        let path = path.as_ref().to_path_buf();

        if let Some(&handle) = self.images.path_lookup.get(&path) {
            return Ok((handle, self.images.get(handle).unwrap()));
        }

        tracing::info!("Loading image: {}", path.display());

        let data = file_system().load(&path)?;
        let mut context = AssetLoadContext { _assets: self };
        let image = Image::from_memory(&mut context, path.clone(), &data)?;
        let handle = self.images.assets.insert(image);

        self.images.path_lookup.insert(path, handle);

        Ok((handle, self.images.get(handle).unwrap()))
    }

    #[inline]
    pub fn get_image(&self, handle: Handle<Image>) -> Option<&Image> {
        self.images.get(handle)
    }
}

pub struct AssetLoadContext<'assets> {
    pub _assets: &'assets mut Assets,
}

struct AssetCache<T: Asset> {
    assets: Storage<T>,
    path_lookup: HashMap<PathBuf, Handle<T>>,
}

impl<T: Asset> Default for AssetCache<T> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
            path_lookup: Default::default(),
        }
    }
}

impl<T: Asset> AssetCache<T> {
    #[inline]
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.assets.get(handle)
    }
}

pub fn load_config<C: From<ConfigLines>>(path: impl AsRef<Path>) -> Result<C, AssetError> {
    let data = file_system().load(path)?;
    let text = String::from_utf8_lossy(&data);
    let config_lines = ConfigLines::parse(&text);
    Ok(C::from(config_lines))
}
