use std::path::{Path, PathBuf};

use shadow_company_tools::smf;

use crate::engine::{
    assets::{AssetStore, Handle},
    renderer::Renderer,
};

use super::{
    model::Model,
    vfs::{FileSystemError, VirtualFileSystem},
};

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("File system error: {0}")]
    FileSystemError(#[from] FileSystemError),

    #[error("Decode error")]
    DecodeError,

    #[error("Image load error: {0}")]
    ImageLoadError(#[from] image::ImageError),

    #[error("{0}")]
    Custom(String),
}

pub struct AssetLoader {
    asset_store: AssetStore,
    fs: VirtualFileSystem,
}

impl AssetLoader {
    pub fn new(asset_store: AssetStore, data_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        Ok(Self {
            asset_store,
            fs: VirtualFileSystem::new(data_dir)?,
        })
    }

    pub fn asset_store(&self) -> &AssetStore {
        &self.asset_store
    }

    pub fn load_smf_model(
        &self,
        path: impl AsRef<Path>,
        renderer: &Renderer,
    ) -> Result<Handle<Model>, AssetError> {
        let smf = self.load_smf(path)?;
        let model = Model::from_smf(&smf, renderer, self)?;
        Ok(self.asset_store.add(model))
    }

    pub fn load_raw(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        Ok(self.fs.load(path)?)
    }

    pub fn load_bmp(&self, path: impl AsRef<Path>) -> Result<image::RgbaImage, AssetError> {
        // Check if there exists a .raw file with alpha data.
        // let raw_data = self.load_raw(path.as_ref().with_extension("raw")).ok();
        // shadow_company_tools::images::load_raw_file(std::io::Cursor::new(raw_data), width, height)

        let bmp = shadow_company_tools::images::load_bmp_file(&mut std::io::Cursor::new(
            self.load_raw(path.as_ref())?,
        ))?;
        let raw = if let Ok(raw_data) = self.load_raw(path.as_ref().with_extension("raw")) {
            Some(shadow_company_tools::images::load_raw_file(
                &mut std::io::Cursor::new(raw_data),
                bmp.width(),
                bmp.height(),
            )?)
        } else {
            None
        };

        if let Some(raw) = raw {
            Ok(shadow_company_tools::images::combine_bmp_and_raw(
                &bmp, &raw,
            ))
        } else {
            Ok(image::DynamicImage::from(bmp).into_rgba8())
        }
    }

    pub fn load_jpeg(&self, path: impl AsRef<Path>) -> Result<image::RgbaImage, AssetError> {
        let data = self.load_raw(path.as_ref())?;

        Ok(
            image::load_from_memory_with_format(data.as_ref(), image::ImageFormat::Jpeg)?
                .into_rgba8(),
        )
    }

    pub fn load_string(&self, path: impl AsRef<Path>) -> Result<String, AssetError> {
        String::from_utf8(self.load_raw(&path)?).map_err(|_| {
            tracing::warn!("Could not load string: {}", path.as_ref().display());
            AssetError::DecodeError
        })
    }

    pub fn load_smf(&self, path: impl AsRef<Path>) -> Result<smf::Model, AssetError> {
        let data = self.load_raw(path)?;
        let mut cursor = std::io::Cursor::new(data);
        smf::Model::read(&mut cursor)
            .map_err(|err| AssetError::FileSystemError(FileSystemError::Io(err)))
    }

    #[inline]
    pub fn enum_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, std::io::Error> {
        self.fs.enum_dir(path)
    }
}
