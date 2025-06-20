use std::{
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use thiserror::Error;

static mut ASSETS: *const Assets = std::ptr::null();

pub fn init_assets(assets: Assets) {
    let b = Box::leak(Box::new(assets)) as &'static Assets;
    unsafe {
        ASSETS = b as *const Assets;
    }
}

pub fn assets() -> &'static Assets {
    unsafe {
        // if ASSETS.is_null() {
        //     panic!("Assets have not been initialized!");
        // }
        &*ASSETS
    }
}

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("File not found ({0})")]
    FileNotFound(PathBuf),

    #[error("Decode error ({0})")]
    Decode(PathBuf),

    #[error("Unsupported asset ({0})")]
    NotSupported(PathBuf),

    #[error("Unknown error ({0})")]
    Unknown(PathBuf, String),
}

impl AssetError {
    pub fn from_io_error(error: std::io::Error, path: &Path) -> Self {
        match error {
            err if err.kind() == std::io::ErrorKind::NotFound => {
                Self::FileNotFound(path.to_path_buf())
            }
            err => Self::Unknown(path.to_path_buf(), err.kind().to_string()),
        }
    }
}

pub struct AssetLoadContext<'a> {
    pub path: &'a Path,
    pub assets: &'a Assets,
}

pub trait AssetType: Sized {
    type Options: Default;

    fn from_raw_with_options(
        raw: &[u8],
        options: Self::Options,
        context: &AssetLoadContext,
    ) -> Result<Self, AssetError>;
}

pub struct Asset<A: AssetType> {
    _assets: Assets,
    asset: Arc<A>,
}

impl<A: AssetType> Deref for Asset<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        self.asset.as_ref()
    }
}

pub trait AssetFileSystem {
    fn load(&self, path: &Path) -> Result<Vec<u8>, AssetError>;
    fn dir(&self, path: &Path) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetError>;
}

#[derive(Clone)]
pub struct Assets {
    file_system: Arc<dyn AssetFileSystem>,
}

impl Default for Assets {
    fn default() -> Self {
        Self {
            file_system: Arc::new(PlatformFileSystem {
                root: std::env::current_dir().expect("Failed to get current working directory"),
            }),
        }
    }
}

impl Assets {
    pub fn with_file_system(file_system: Arc<dyn AssetFileSystem>) -> Self {
        Self { file_system }
    }

    #[inline]
    pub fn load<A: AssetType>(&self, path: impl AsRef<Path>) -> Result<Asset<A>, AssetError> {
        self.load_with_options(path, A::Options::default())
    }

    pub fn load_with_options<A: AssetType>(
        &self,
        path: impl AsRef<Path>,
        options: A::Options,
    ) -> Result<Asset<A>, AssetError> {
        let data = self.file_system.load(path.as_ref())?;
        let load_context = AssetLoadContext {
            path: path.as_ref(),
            assets: self,
        };
        Ok(Asset {
            _assets: self.clone(),
            asset: Arc::new(A::from_raw_with_options(&data, options, &load_context)?),
        })
    }

    #[inline]
    pub fn load_direct<A: AssetType>(&self, path: impl AsRef<Path>) -> Result<A, AssetError> {
        self.load_direct_with_options(path, A::Options::default())
    }

    pub fn load_direct_with_options<A: AssetType>(
        &self,
        path: impl AsRef<Path>,
        options: A::Options,
    ) -> Result<A, AssetError> {
        let data = self.file_system.load(path.as_ref())?;
        let load_context = AssetLoadContext {
            path: path.as_ref(),
            assets: self,
        };
        A::from_raw_with_options(&data, options, &load_context)
    }

    pub fn dir(&self, path: &Path) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetError> {
        self.file_system.dir(path)
    }
}

pub struct PlatformFileSystem {
    root: PathBuf,
}

impl PlatformFileSystem {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl AssetFileSystem for PlatformFileSystem {
    fn load(&self, path: &Path) -> Result<Vec<u8>, AssetError> {
        std::fs::read(self.root.join(path)).map_err(|err| AssetError::from_io_error(err, path))
    }

    fn dir(&self, path: &Path) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetError> {
        let mut files = vec![];
        for entry in std::fs::read_dir(self.root.join(path))
            .map_err(|err| AssetError::from_io_error(err, path))?
        {
            let entry = entry.map_err(|err| AssetError::from_io_error(err, path))?;
            files.push(path.join(entry.file_name()));
        }
        Ok(Box::new(files.into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn asset_error_from_io_error() {
        let error = AssetError::from_io_error(
            std::io::Error::new(std::io::ErrorKind::NotFound, "not_found"),
            &PathBuf::from("abc"),
        );

        assert!(matches!(error, AssetError::FileNotFound(path) if path == PathBuf::from("abc")));

        let error = AssetError::from_io_error(
            std::io::Error::new(std::io::ErrorKind::NotADirectory, "not_a_directory"),
            &PathBuf::from("abc"),
        );
        assert!(matches!(error, AssetError::Unknown(path, _) if path == PathBuf::from("abc")));
    }

    struct TestFileSystem {
        files: HashMap<PathBuf, Vec<u8>>,
    }

    impl AssetFileSystem for TestFileSystem {
        fn load(&self, path: &Path) -> Result<Vec<u8>, AssetError> {
            match self.files.get(path) {
                None => Err(AssetError::FileNotFound(path.to_path_buf())),
                Some(data) => Ok(data.clone()),
            }
        }

        fn dir(&self, _path: &Path) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetError> {
            let files = self.files.keys().cloned().collect::<Vec<_>>();
            Ok(Box::new(files.into_iter()))
        }
    }

    #[test]
    fn file_system() {
        let file_system = Arc::new(TestFileSystem {
            files: {
                let mut files = HashMap::default();
                files.insert(PathBuf::from("a"), "1234".to_string().into_bytes());
                files
            },
        });

        let data = file_system.load(&PathBuf::from("a")).unwrap();
        assert_eq!(&data, b"1234");

        assert!(matches!(
            file_system.load(&PathBuf::from("b")),
            Err(AssetError::FileNotFound(path)) if path == PathBuf::from("b")
        ));
    }

    struct IntData {
        data: i32,
    }

    impl AssetType for IntData {
        type Options = ();

        fn from_raw_with_options(
            raw: &[u8],
            _options: Self::Options,
            load_context: &AssetLoadContext,
        ) -> Result<Self, AssetError> {
            String::from_utf8_lossy(raw)
                .parse::<i32>()
                .map_err(|_| AssetError::Decode(load_context.path.to_path_buf()))
                .map(|data| Self { data })
        }
    }

    #[test]
    fn load_an_asset() {
        let file_system = TestFileSystem {
            files: {
                let mut files = HashMap::default();
                files.insert(PathBuf::from("a"), "1234".to_string().into_bytes());
                files
            },
        };

        let assets = Assets::with_file_system(Arc::new(file_system));
        let result = assets.load::<IntData>(&PathBuf::from("a")).unwrap();
        assert_eq!(result.data, 1234);
    }

    struct IntSubData {
        data: Asset<IntData>,
    }

    impl AssetType for IntSubData {
        type Options = ();

        fn from_raw_with_options(
            raw: &[u8],
            _options: Self::Options,
            load_context: &AssetLoadContext,
        ) -> Result<Self, AssetError> {
            let sub_resource_name = String::from_utf8_lossy(raw);
            load_context
                .assets
                .load::<IntData>(sub_resource_name.trim())
                .map(|data| Self { data })
        }
    }

    #[test]
    fn load_sub_asset() {
        let file_system = TestFileSystem {
            files: {
                let mut files = HashMap::default();
                files.insert(PathBuf::from("a"), "b".to_string().into_bytes());
                files.insert(PathBuf::from("b"), "1234".to_string().into_bytes());
                files
            },
        };

        let assets = Assets::with_file_system(Arc::new(file_system));
        let result = assets.load::<IntSubData>(&PathBuf::from("a")).unwrap();
        assert_eq!(result.data.data, 1234);
    }

    #[test]
    fn dir() {
        let file_system = TestFileSystem {
            files: {
                let mut files = HashMap::default();
                files.insert(PathBuf::from("a"), "b".to_string().into_bytes());
                files.insert(PathBuf::from("b"), "1234".to_string().into_bytes());
                files
            },
        };

        let assets = Assets::with_file_system(Arc::new(file_system));
        let files = assets.dir(&PathBuf::from("")).unwrap().collect::<Vec<_>>();
        assert!(files.contains(&PathBuf::from("a")));
        assert!(files.contains(&PathBuf::from("b")));
    }
}
