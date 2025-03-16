use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io::{Read, Seek},
    path::{Path, PathBuf},
    rc::Rc,
};

use shadow_company_tools::common::decrypt_buf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("The path was not found: {0}")]
    FileNotFound(PathBuf),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
}

pub trait FileSystemLayer {
    fn load(&self, path: &Path) -> Result<Vec<u8>, FileSystemError>;
    fn dir(&self, prefix: &Path) -> Result<Vec<PathBuf>, FileSystemError>;
}

#[derive(Default)]
pub struct FileSystem {
    layers: Vec<Box<dyn FileSystemLayer>>,
}

impl FileSystem {
    pub fn push_layer<L>(&mut self, layer: L)
    where
        L: FileSystemLayer + 'static,
    {
        self.layers.push(Box::new(layer));
    }

    pub fn load(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        for layer in &self.layers {
            if let Ok(data) = layer.load(path) {
                return Ok(data);
            }
        }

        Err(FileSystemError::FileNotFound(path.to_path_buf()))
    }

    pub fn dir(&self, prefix: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let mut dirs: HashSet<PathBuf> = HashSet::default();

        for layer in &self.layers {
            if let Ok(data) = layer.dir(prefix) {
                dirs.extend(data);
            } else {
                tracing::warn!("Could not get directory listing for layer");
            }
        }

        Ok(Vec::from_iter(dirs.drain()))
    }
}

pub struct OsFileSystemLayer {
    root: PathBuf,
}

impl OsFileSystemLayer {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }
}

impl FileSystemLayer for OsFileSystemLayer {
    fn load(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        let path = self.root.join(path);
        Ok(std::fs::read(path)?)
    }

    fn dir(&self, prefix: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let root = self.root.join(prefix);
        Ok(walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    if entry.file_type().is_file() {
                        Some(pathdiff::diff_paths(entry.into_path(), &self.root).unwrap())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect())
    }
}

#[derive(Debug)]
struct GutFileEntry {
    offset: u64,
    size: u64,
    encoded: bool,
}

type GutFile = HashMap<PathBuf, GutFileEntry>;

pub struct GutFileSystemLayer {
    root: PathBuf,

    gut_files: RefCell<HashMap<PathBuf, Rc<GutFile>>>,
}

impl GutFileSystemLayer {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),

            gut_files: RefCell::new(HashMap::default()),
        }
    }

    fn get_gut_file(&self, gut_file_path: &Path) -> Result<Rc<GutFile>, FileSystemError> {
        if let Some(gut_file) = self.gut_files.borrow().get(gut_file_path) {
            return Ok(Rc::clone(gut_file));
        }

        if !gut_file_path.exists() {
            return Err(FileSystemError::FileNotFound(gut_file_path.to_path_buf()));
        }

        let mut reader = std::io::Cursor::new(std::fs::read(gut_file_path)?);
        let gut_file =
            shadow_company_tools::gut::GutFile::open(&mut reader).map_err(|err| match err {
                shadow_company_tools::gut::GutError::Io(error) => FileSystemError::Io(error),
            })?;

        let mut entries = GutFile::default();
        gut_file.entries().for_each(|entry| {
            let path = PathBuf::from(
                entry
                    .name
                    .chars()
                    .map(|ch| {
                        if ch == '\\' {
                            std::path::MAIN_SEPARATOR
                        } else {
                            ch
                        }
                    })
                    .collect::<String>(),
            );
            let entry = GutFileEntry {
                offset: entry.offset,
                size: entry.size,
                encoded: entry.is_plain_text,
            };
            entries.insert(path, entry);
        });

        // Try to load the .gut file.
        let gut_file = Rc::new(entries);

        self.gut_files
            .borrow_mut()
            .insert(gut_file_path.to_path_buf(), Rc::clone(&gut_file));

        Ok(gut_file)
    }

    fn gut_file_path_for(&self, path: &Path) -> PathBuf {
        self.root
            .join(path.components().next().unwrap())
            .with_extension("gut")
    }
}

impl FileSystemLayer for GutFileSystemLayer {
    fn load(&self, path: &Path) -> Result<Vec<u8>, FileSystemError> {
        let gut_file_path = self.gut_file_path_for(path);

        let gut_file = self.get_gut_file(&gut_file_path)?;

        let path = PathBuf::from(path.to_string_lossy().to_lowercase());

        let Some(entry) = gut_file.get(&path) else {
            return Err(FileSystemError::FileNotFound(path));
        };

        let mut buf = vec![0_u8; entry.size as usize];
        let mut f = std::fs::File::open(&gut_file_path)?;
        f.seek(std::io::SeekFrom::Start(entry.offset))?;
        f.read_exact(&mut buf)?;

        if entry.encoded {
            decrypt_buf(&mut buf);
        }

        Ok(buf)
    }

    fn dir(&self, prefix: &Path) -> Result<Vec<PathBuf>, FileSystemError> {
        let gut_file_path = self.gut_file_path_for(prefix);

        let gut_file = self.get_gut_file(&gut_file_path)?;

        Ok(gut_file
            .keys()
            .filter_map(|path| path.starts_with(prefix).then_some(path.clone()))
            .collect())
    }
}
