use std::path::PathBuf;

use crate::engine::assets::AssetError;

use super::AssetLoadContext;

pub trait Asset: Sized + 'static {
    fn from_memory(
        context: &mut AssetLoadContext,
        path: PathBuf,
        data: &[u8],
    ) -> Result<Self, AssetError>;
}
