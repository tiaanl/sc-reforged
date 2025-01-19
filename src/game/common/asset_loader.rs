use std::{
    any::TypeId,
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use shadow_company_tools::smf;

use crate::{
    engine::{
        assets::{AssetStore, Handle},
        renderer::Renderer,
    },
    game::config::ImageDefs,
    Asset,
};

use super::{
    assets::Image,
    mesh_renderer::BlendMode,
    model::Model,
    vfs::{FileSystemError, VirtualFileSystem},
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

#[derive(Default)]
struct PathCache {
    paths: HashMap<TypeId, HashMap<PathBuf, u64>>,
}

impl PathCache {
    fn insert<A>(&mut self, path: PathBuf, handle: Handle<A>)
    where
        A: Asset + Send + Sync + 'static,
    {
        let typed_paths = self.paths.entry(TypeId::of::<A>()).or_default();
        typed_paths.insert(path, handle.as_raw());
    }

    fn get<A>(&self, path: impl AsRef<Path>) -> Option<Handle<A>>
    where
        A: Asset + Send + Sync + 'static,
    {
        self.paths.get(&TypeId::of::<A>()).map(|typed_paths| {
            typed_paths
                .get(path.as_ref())
                .map(|id| Handle::from_raw(*id))
        })?
    }
}

pub struct AssetLoader {
    /// The backend storing all the loaded assets and associated handles.
    asset_store: AssetStore,
    /// The file system we use to load data from the OS.
    fs: VirtualFileSystem,
    /// A cache of paths to handles we use to avoid loading duplicate data.
    paths: RefCell<PathCache>,
    /// A cache of the image definitions used to load images.
    image_defs: ImageDefs,
}

impl AssetLoader {
    pub fn new(asset_store: AssetStore, data_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let mut s = Self {
            asset_store,
            fs: VirtualFileSystem::new(data_dir)?,
            paths: RefCell::new(PathCache::default()),
            image_defs: ImageDefs::default(),
        };

        let image_defs = s
            .load_config::<ImageDefs>(r"config\image_defs.txt")
            .unwrap();
        s.image_defs = image_defs;

        Ok(s)
    }

    pub fn asset_store(&self) -> &AssetStore {
        &self.asset_store
    }

    pub fn load_smf_direct(&self, path: impl AsRef<Path>) -> Result<smf::Model, AssetError> {
        let mut reader = std::io::Cursor::new(self.load_raw(&path)?);
        Ok(smf::Model::read(&mut reader)?)
    }

    pub fn load_smf(
        &self,
        path: impl AsRef<Path>,
        renderer: &Renderer,
    ) -> Result<Handle<Model>, AssetError> {
        self.load_cached(path, |asset_loader, path| {
            // We convert the .smf to our own model data, so we can just throw it away and not
            // store it in the asset cache.
            let raw = self.fs.load(path)?;
            let mut reader = std::io::Cursor::new(raw);
            let smf = smf::Model::read(&mut reader)
                .map_err(|err| AssetError::FileSystemError(FileSystemError::Io(err)))?;

            Model::from_smf(&smf, renderer, asset_loader)
        })
    }

    pub fn load_raw(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        Ok(self.fs.load(path)?)
    }

    pub fn load_bmp(&self, path: impl AsRef<Path>) -> Result<Handle<Image>, AssetError> {
        self.load_cached(path, |asset_loader, path| {
            let color_keyd = path
                .file_name()
                .map(|n| n.to_string_lossy().contains("_ck"))
                .unwrap_or(false);

            let bmp = shadow_company_tools::images::load_bmp_file(
                &mut std::io::Cursor::new(asset_loader.fs.load(path)?),
                color_keyd,
            )?;

            let raw = if let Ok(raw_data) = asset_loader.fs.load(path.with_extension("raw")) {
                // debug_assert!(!color_keyd);
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
                Image::from_rgba(image::DynamicImage::from(bmp).into_rgba8(), BlendMode::None)
            })
        })
    }

    pub fn load_jpeg(&self, path: impl AsRef<Path>) -> Result<Handle<Image>, AssetError> {
        self.load_cached(path, |asset_loader, path| {
            let data = asset_loader.load_raw(path)?;
            let image =
                image::load_from_memory_with_format(data.as_ref(), image::ImageFormat::Jpeg)?;
            Ok(Image::from_rgba(image.into_rgba8(), BlendMode::None))
        })
    }

    pub fn load_config<C>(&self, path: impl AsRef<Path>) -> Result<C, AssetError>
    where
        C: TryFrom<String, Error = AssetError>,
    {
        let raw = self.fs.load(&path)?;
        let s = String::from_utf8(raw).map_err(|_| {
            tracing::warn!("Could not load string: {}", path.as_ref().display());
            AssetError::DecodeError
        })?;
        C::try_from(s)
    }

    // pub fn load_string(&self, path: impl AsRef<Path>) -> Result<String, AssetError> {
    //     String::from_utf8(self.load_raw(&path)?).map_err(|_| {
    //         tracing::warn!("Could not load string: {}", path.as_ref().display());
    //         AssetError::DecodeError
    //     })
    // }

    #[inline]
    pub fn enum_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, std::io::Error> {
        self.fs.enum_dir(path)
    }

    fn load_cached<P, A, F>(&self, path: P, create: F) -> Result<Handle<A>, AssetError>
    where
        P: AsRef<Path>,
        A: Asset + Send + Sync + 'static,
        F: Fn(&AssetLoader, &Path) -> Result<A, AssetError>,
    {
        debug_assert!(!path.as_ref().is_absolute());

        if let Some(handle) = self.paths.borrow().get(path.as_ref()) {
            return Ok(handle);
        }

        let asset = create(self, path.as_ref())?;
        let handle = self.asset_store.add(asset);
        self.paths
            .borrow_mut()
            .insert(path.as_ref().to_path_buf(), handle);

        Ok(handle)
    }
}
