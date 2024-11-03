use std::path::{Path, PathBuf};

use crate::engine::arena::{Arena, Handle};

pub type TextureHandle = Handle<wgpu::BindGroup>;

#[derive(Default)]
pub struct Textures {
    bind_groups: Arena<wgpu::BindGroup>,
    name_lookup: ahash::HashMap<PathBuf, TextureHandle>,
}

impl Textures {
    pub fn get(&self, handle: &TextureHandle) -> Option<&wgpu::BindGroup> {
        self.bind_groups.get(handle)
    }

    pub fn get_by_path_or_insert(
        &mut self,
        path: impl AsRef<Path>,
        loader: impl Fn(&Path) -> Option<wgpu::BindGroup>,
    ) -> Result<TextureHandle, ()> {
        match self.name_lookup.get(path.as_ref()) {
            Some(handle) => Ok(handle.clone()),
            None => {
                // Let the loader create a new wgpu::BindGroup.
                if let Some(bind_group) = loader(path.as_ref()) {
                    let new_handle = self.bind_groups.insert(bind_group);
                    self.name_lookup
                        .insert(path.as_ref().to_path_buf(), new_handle.clone());
                    Ok(new_handle)
                } else {
                    // The loader could not create a new bind group.
                    return Err(());
                }
            }
        }
    }
}
