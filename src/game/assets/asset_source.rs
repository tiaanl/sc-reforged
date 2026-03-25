use std::path::PathBuf;

pub enum AssetSource {
    Generated,
    FileSystem(PathBuf),
}
