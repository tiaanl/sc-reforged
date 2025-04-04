use std::{
    any::TypeId,
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
};

use shadow_company_tools::{bmf, smf};

use crate::{
    Asset,
    engine::{
        assets::{AssetStore, Handle, resources::Resources},
        renderer::Renderer,
    },
    game::config::ImageDefs,
};

use super::{
    file_system::{FileSystem, FileSystemError, GutFileSystemLayer, OsFileSystemLayer},
    image::Image,
    mesh_renderer::BlendMode,
    model::Model,
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

    fn get<A>(&self, path: &Path) -> Option<Handle<A>>
    where
        A: Asset + Send + Sync + 'static,
    {
        self.paths
            .get(&TypeId::of::<A>())
            .map(|typed_paths| typed_paths.get(path).map(|id| Handle::from_raw(*id)))?
    }
}

pub struct AssetLoader {
    /// The backend storing all the loaded assets and associated handles.
    asset_store: AssetStore,
    /// The file system we use to load data from the OS.
    file_system: FileSystem,
    /// A cache of paths to handles we use to avoid loading duplicate data.
    paths: RefCell<PathCache>,
    /// A cache of the image definitions used to load images.
    image_defs: ImageDefs,
}

unsafe impl Send for AssetLoader {}
unsafe impl Sync for AssetLoader {}

impl AssetLoader {
    pub fn new(asset_store: AssetStore, data_dir: &Path) -> std::io::Result<Self> {
        let root = data_dir.canonicalize()?;

        let mut file_system = FileSystem::default();
        file_system.push_layer(OsFileSystemLayer::new(&root));
        file_system.push_layer(GutFileSystemLayer::new(&root));

        let mut s = Self {
            asset_store,
            file_system,
            paths: RefCell::new(PathCache::default()),
            image_defs: ImageDefs::default(),
        };

        let image_defs = s
            .load_config::<ImageDefs>(&PathBuf::from("config").join("image_defs.txt"))
            .unwrap();
        s.image_defs = image_defs;

        Ok(s)
    }

    pub fn asset_store(&self) -> &AssetStore {
        &self.asset_store
    }

    pub fn load_smf_direct(&self, path: &Path) -> Result<smf::Model, AssetError> {
        let mut reader = std::io::Cursor::new(self.load_raw(path)?);
        Ok(smf::Model::read(&mut reader)?)
    }

    pub fn load_smf(
        &self,
        path: &Path,
        renderer: &Renderer,
        resources: &Resources,
    ) -> Result<Handle<Model>, AssetError> {
        self.load_cached(path, |asset_loader, path| {
            // We convert the .smf to our own model data, so we can just throw it away and not
            // store it in the asset cache.
            let raw = self.file_system.load(path)?;
            let mut reader = std::io::Cursor::new(raw);
            let smf = smf::Model::read(&mut reader)
                .map_err(|err| AssetError::FileSystemError(FileSystemError::Io(err)))?;

            Model::from_smf(&smf, renderer, resources, asset_loader.asset_store())
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

    pub fn load_bmp(&self, path: &Path) -> Result<Handle<Image>, AssetError> {
        self.load_cached(path, |asset_loader, path| {
            asset_loader.load_bmp_direct(path)
        })
    }

    pub fn load_jpeg(&self, path: &Path) -> Result<Handle<Image>, AssetError> {
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
