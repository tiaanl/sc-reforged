use crate::engine::assets::resources::{ResourceLoadContext, ResourceType};

pub struct TextFile(String);

impl ResourceType for TextFile {
    fn from_data(data: Vec<u8>, _context: &ResourceLoadContext) -> Result<Self, ()> {
        Ok(Self(String::from_utf8(data).map_err(|_| ())?))
    }
}

impl TextFile {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<TextFile> for String {
    fn from(value: TextFile) -> Self {
        value.0.clone()
    }
}
