mod data_dir;
mod text_file;

pub use data_dir::*;
pub use text_file::*;

use crate::engine::assets::{AssetError, AssetLoadContext, AssetType};
use shadow_company_tools::{bmf, smf};

pub trait Config: AssetType {
    fn from_string(str: &str) -> Result<Self, AssetError>;
}

impl<C> AssetType for C
where
    C: Config,
{
    type Options = ();

    fn from_raw_with_options(
        raw: &[u8],
        _options: Self::Options,
        _load_context: &AssetLoadContext,
    ) -> Result<Self, AssetError> {
        Self::from_string(&String::from_utf8_lossy(raw))
    }
}

impl AssetType for Vec<u8> {
    type Options = ();

    fn from_raw_with_options(
        raw: &[u8],
        _options: Self::Options,
        _load_context: &AssetLoadContext,
    ) -> Result<Self, AssetError> {
        Ok(raw.to_vec())
    }
}

impl AssetType for smf::Model {
    type Options = ();

    fn from_raw_with_options(
        raw: &[u8],
        _options: Self::Options,
        context: &AssetLoadContext,
    ) -> Result<Self, AssetError> {
        let mut reader = std::io::Cursor::new(raw);
        smf::Model::read(&mut reader).map_err(|err| AssetError::from_io_error(err, context.path))
    }
}

impl AssetType for bmf::Motion {
    type Options = ();

    fn from_raw_with_options(
        raw: &[u8],
        _options: Self::Options,
        context: &AssetLoadContext,
    ) -> Result<Self, AssetError> {
        let mut data = std::io::Cursor::new(raw);
        bmf::Motion::read(&mut data).map_err(|err| AssetError::from_io_error(err, context.path))
    }
}
