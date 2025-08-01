use std::{
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use ahash::{HashMap, HashSet};
use thiserror::Error;
use walkdir::WalkDir;

use crate::global;

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("The path was not found: {0}")]
    FileNotFound(PathBuf),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct FileSystem {
    root_dir: PathBuf,
    gut_files: HashMap<String, GutFile>,
}

struct GutFile {
    path: PathBuf,
    entries: HashMap<String, GutEntry>,
}

impl GutFile {
    fn from_path(path: impl AsRef<Path>) -> Result<Self, FileSystemError> {
        use shadow_company_tools::gut;

        let file = std::fs::File::open(path.as_ref())?;
        let gut_file = match gut::GutFile::open(&mut std::io::BufReader::new(file)) {
            Ok(gut_file) => gut_file,
            Err(err) => match err {
                gut::GutError::Io(err) => {
                    return Err(FileSystemError::Io(err));
                }
            },
        };

        let mut entries: HashMap<String, GutEntry> = HashMap::default();
        gut_file.entries().for_each(|entry| {
            // Although the rule is that all paths in a .gut file are lower case, we enforce it.
            entries.insert(
                entry.name.to_ascii_lowercase().to_string(),
                GutEntry {
                    offset: entry.offset,
                    size: entry.size,
                    is_plain_text: entry.is_plain_text,
                },
            );
        });

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            entries,
        })
    }

    fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        // Paths in a .gut file are all lower case and use the `\`` separator.
        let lower_path = path
            .as_ref()
            .to_string_lossy()
            .to_ascii_lowercase()
            .replace(std::path::MAIN_SEPARATOR, "\\")
            .to_string();

        let Some(entry) = self.entries.get(&lower_path) else {
            return Err(FileSystemError::FileNotFound(path.as_ref().to_path_buf()));
        };

        let mut file = std::fs::File::open(&self.path)?;
        let a = file.seek(SeekFrom::Start(entry.offset))?;
        if a != entry.offset {
            return Err(FileSystemError::Io(ErrorKind::InvalidData.into()));
        }

        let mut data = vec![0u8; entry.size as usize];
        file.read_exact(&mut data)?;

        if entry.is_plain_text {
            shadow_company_tools::common::decrypt_buf(&mut data);
        }

        Ok(data)
    }
}

#[derive(Clone)]
struct GutEntry {
    pub offset: u64,
    pub size: u64,
    pub is_plain_text: bool,
}

impl FileSystem {
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        let mut gut_files = HashMap::default();

        WalkDir::new(&root_dir)
            .into_iter()
            .filter_map(|entry| match entry {
                Ok(dir_entry) => {
                    if let Some(ext) = dir_entry.path().extension() {
                        if ext.eq_ignore_ascii_case("gut") {
                            Some(dir_entry.into_path())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None,
            })
            .for_each(|path| {
                if let Ok(gut_entry) = GutFile::from_path(&path) {
                    if let Some(name) = path.file_stem() {
                        gut_files.insert(name.to_string_lossy().to_string(), gut_entry);
                    }
                }
            });

        Self {
            root_dir: root_dir
                .as_ref()
                .canonicalize()
                .expect("Could not canonicalize root path"),
            gut_files,
        }
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        if let Some(gut_file) = self.gut_file_for_path(&path) {
            return gut_file.load(path);
        }

        let full_path = self.root_dir.join(path.as_ref());

        let mut file = std::fs::File::open(full_path).map_err(|err| {
            if let ErrorKind::NotFound = err.kind() {
                FileSystemError::FileNotFound(path.as_ref().to_path_buf())
            } else {
                FileSystemError::Io(err)
            }
        })?;
        let mut data = Vec::default();
        let _bytes_read = file.read_to_end(&mut data)?;

        Ok(data)
    }

    pub fn dir(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<impl Iterator<Item = PathBuf>, FileSystemError> {
        // TODO: Enumerate the root directory?
        let mut result: HashSet<PathBuf> = HashSet::default();

        if let Some(gut_file) = self.gut_file_for_path(&root) {
            let search = root
                .as_ref()
                .to_string_lossy()
                .to_ascii_lowercase()
                .replace(std::path::MAIN_SEPARATOR, "\\");
            for gut_path in gut_file.entries.keys() {
                if gut_path.starts_with(&search) {
                    // Convert the separators to OS separators again.
                    result.insert(PathBuf::from(
                        gut_path.replace(r"\", std::path::MAIN_SEPARATOR_STR),
                    ));
                }
            }
        }

        let root = self.root_dir.join(root);

        walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(|entry| {
                if let Ok(entry) = entry {
                    if entry.file_type().is_file() {
                        Some(pathdiff::diff_paths(entry.into_path(), &self.root_dir).unwrap())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .for_each(|path| {
                result.insert(path);
            });

        Ok(result.into_iter())
    }

    fn gut_file_for_path(&self, path: impl AsRef<Path>) -> Option<&GutFile> {
        let component = path.as_ref().components().next()?;
        self.gut_files.get(component.as_os_str().to_str()?)
    }
}

global!(FileSystem, scoped_file_system, file_system);
