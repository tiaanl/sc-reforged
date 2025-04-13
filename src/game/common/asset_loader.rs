use std::{
    any::{Any, TypeId},
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use parking_lot::RwLock;
use shadow_company_tools::{bmf, smf};

use crate::game::config::ImageDefs;

use super::{
    file_system::{FileSystem, FileSystemError, GutFileSystemLayer, OsFileSystemLayer},
    image::{BlendMode, Image},
};

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("File system error: {0}")]
    FileSystemError(#[from] FileSystemError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Decode error")]
    DecodeError,

    #[error("Image load error: {0}")]
    ImageLoadError(#[from] image::ImageError),

    #[error("{0}")]
    Custom(String),
}

pub trait Asset {}

pub struct AssetLoader {
    /// A cache holding all loaded assets.
    asset_cache: AssetCache,
    /// The file system we use to load data from the OS.
    file_system: FileSystem,
    /// A cache of the image definitions used to load images.
    image_defs: ImageDefs,
}

unsafe impl Send for AssetLoader {}
unsafe impl Sync for AssetLoader {}

impl AssetLoader {
    pub fn new(data_dir: &Path) -> std::io::Result<Self> {
        let root = data_dir.canonicalize()?;

        let asset_cache = AssetCache::default();

        let mut file_system = FileSystem::default();
        file_system.push_layer(OsFileSystemLayer::new(&root));
        file_system.push_layer(GutFileSystemLayer::new(&root));

        let mut s = Self {
            asset_cache,
            file_system,
            image_defs: ImageDefs::default(),
        };

        let image_defs = s
            .load_config::<ImageDefs>(&PathBuf::from("config").join("image_defs.txt"))
            .unwrap();
        s.image_defs = image_defs;

        Ok(s)
    }

    pub fn load_smf_direct(&self, path: &Path) -> Result<smf::Model, AssetError> {
        let mut reader = std::io::Cursor::new(self.load_raw(path)?);
        Ok(smf::Model::read(&mut reader)?)
    }

    pub fn load_smf(&self, path: &Path) -> Result<Arc<smf::Model>, AssetError> {
        self.load_cached(path, |_, path| {
            // We convert the .smf to our own model data, so we can just throw it away and not
            // store it in the asset cache.
            let raw = self.file_system.load(path)?;
            let mut reader = std::io::Cursor::new(raw);
            smf::Model::read(&mut reader)
                .map_err(|err| AssetError::FileSystemError(FileSystemError::Io(err)))
        })
    }

    pub fn load_raw(&self, path: &Path) -> Result<Vec<u8>, AssetError> {
        Ok(self.file_system.load(path)?)
    }

    pub fn load_bmp_direct(&self, path: &Path) -> Result<Image, AssetError> {
        let color_keyd = path
            .file_name()
            .map(|n| n.to_string_lossy().contains("_ck"))
            .unwrap_or(false);

        let bmp = shadow_company_tools::images::load_bmp_file(
            &mut std::io::Cursor::new(self.file_system.load(path)?),
            color_keyd,
        )?;

        let raw = if let Ok(raw_data) = self.file_system.load(&path.with_extension("raw")) {
            Some(shadow_company_tools::images::load_raw_file(
                &mut std::io::Cursor::new(raw_data),
                bmp.width(),
                bmp.height(),
            )?)
        } else {
            None
        };

        Ok(if color_keyd {
            Image::from_rgba(
                image::DynamicImage::from(bmp).into_rgba8(),
                BlendMode::ColorKeyed,
            )
        } else if let Some(raw) = raw {
            Image::from_rgba(
                shadow_company_tools::images::combine_bmp_and_raw(&bmp, &raw),
                BlendMode::Alpha,
            )
        } else {
            Image::from_rgba(
                image::DynamicImage::from(bmp).into_rgba8(),
                BlendMode::Opaque,
            )
        })
    }

    pub fn load_bmp(&self, path: &Path) -> Result<Arc<Image>, AssetError> {
        self.load_cached(path, |asset_loader, path| {
            asset_loader.load_bmp_direct(path)
        })
    }

    pub fn load_jpeg(&self, path: &Path) -> Result<Arc<Image>, AssetError> {
        self.load_cached(path, |asset_loader, path| {
            let data = asset_loader.load_raw(path)?;
            let image =
                image::load_from_memory_with_format(data.as_ref(), image::ImageFormat::Jpeg)?;
            Ok(Image::from_rgba(image.into_rgba8(), BlendMode::Opaque))
        })
    }

    pub fn load_bmf_direct(&self, path: &Path) -> Result<bmf::Motion, AssetError> {
        let mut data = std::io::Cursor::new(self.file_system.load(path)?);
        Ok(bmf::Motion::read(&mut data)?)
    }

    pub fn load_config<C>(&self, path: &Path) -> Result<C, AssetError>
    where
        C: TryFrom<String, Error = AssetError>,
    {
        let raw = self.file_system.load(path)?;
        let s = String::from_utf8(raw).map_err(|_| {
            tracing::warn!("Could not load string: {}", path.display());
            AssetError::DecodeError
        })?;

        C::try_from(s)
    }

    #[inline]
    pub fn enum_dir(&self, path: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        self.file_system.dir(path)
    }

    fn load_cached<P, A, F>(&self, path: P, mut create: F) -> Result<Arc<A>, AssetError>
    where
        P: AsRef<Path>,
        A: Asset + Send + Sync + 'static,
        F: FnMut(&AssetLoader, &Path) -> Result<A, AssetError>,
    {
        debug_assert!(!path.as_ref().is_absolute());

        // Return the asset if it exists in the cache already.
        if let Some(asset) = self.asset_cache.get::<A>(path.as_ref()) {
            return Ok(asset);
        }

        let asset = create(self, path.as_ref())?;
        Ok(self.asset_cache.insert(path.as_ref().to_path_buf(), asset))
    }
}

struct TypedAssetCache<A: Asset> {
    cache: HashMap<PathBuf, Arc<A>>,
}

impl<A: Asset> TypedAssetCache<A> {
    fn insert(&mut self, path: PathBuf, asset: A) -> Arc<A> {
        let asset = Arc::new(asset);
        self.cache.insert(path, Arc::clone(&asset));
        asset
    }

    fn get(&self, path: &PathBuf) -> Option<Arc<A>> {
        self.cache.get(path).cloned()
    }
}

impl<A: Asset> Default for TypedAssetCache<A> {
    fn default() -> Self {
        Self {
            cache: Default::default(),
        }
    }
}

#[derive(Default)]
struct AssetCache {
    cache: RwLock<HashMap<TypeId, Box<dyn Any>>>,
}

impl AssetCache {
    fn insert<A: Asset + 'static>(&self, path: PathBuf, asset: A) -> Arc<A> {
        let type_id = TypeId::of::<A>();

        let mut storage = self.cache.write();
        let typed_cache = storage
            .entry(type_id)
            .or_insert_with(|| Box::new(TypedAssetCache::<A>::default()))
            .downcast_mut::<TypedAssetCache<A>>()
            .expect("Failed to downcast to TypedAssetCache");

        typed_cache.insert(path, asset)
    }

    fn get<A: Asset + 'static>(&self, path: &Path) -> Option<Arc<A>> {
        let type_id = TypeId::of::<A>();

        let storage = self.cache.read();
        let typed_cache = storage
            .get(&type_id)?
            .downcast_ref::<TypedAssetCache<A>>()
            .expect("Failed to downcast to TypedAssetCache");

        typed_cache.get(&path.to_path_buf())
    }
}
