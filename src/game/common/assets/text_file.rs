use crate::engine::assets::AssetType;

pub struct TextFile(String);

impl AssetType for TextFile {
    type Options = ();

    fn from_raw_with_options(
        raw: &[u8],
        _options: Self::Options,
        _load_context: &crate::engine::assets::AssetLoadContext,
    ) -> Result<Self, crate::engine::assets::AssetError> {
        Ok(Self(String::from_utf8_lossy(raw).to_string()))
    }
}

impl std::ops::Deref for TextFile {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TextFile> for String {
    fn from(value: TextFile) -> Self {
        value.0.clone()
    }
}
