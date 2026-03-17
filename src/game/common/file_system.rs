use std::{
    io::{ErrorKind, Read},
    path::{Path, PathBuf},
};

use ahash::{HashMap, HashSet};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("The path was not found: {0}")]
    FileNotFound(PathBuf),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct FileSystem {
    root_dir: PathBuf,
    gut_files: HashMap<String, GutFile>,
}

struct GutFile {
    file: std::fs::File,
    entries: HashMap<String, GutEntry>,
}

impl GutFile {
    fn from_path(path: impl AsRef<Path>) -> Result<Self, FileSystemError> {
        use shadow_company_tools::gut;

        let mut file = std::fs::File::open(path.as_ref())?;
        let gut_file = match gut::GutFile::open(&mut std::io::BufReader::new(&mut file)) {
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

        Ok(Self { file, entries })
    }

    /// Reads a single file entry.
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

        let mut data = vec![0u8; entry.size as usize];
        read_exact_at(&self.file, &mut data, entry.offset)?;

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
    /// Builds a virtual file system rooted at the game data directory.
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
                if let Ok(gut_entry) = GutFile::from_path(&path)
                    && let Some(name) = path.file_stem()
                {
                    gut_files.insert(name.to_string_lossy().to_string(), gut_entry);
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

    /// Loads a file either from disk or from a mounted .gut archive.
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        let full_path = self.root_dir.join(path.as_ref());

        if !full_path.exists()
            && let Some(gut_file) = self.gut_file_for_path(&path)
        {
            return gut_file.load(path);
        }

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

    /// Returns all files below `root`, combining external files and matching .gut entries.
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

/// Reads exactly `buf.len()` bytes from `file` starting at `offset` without changing a shared cursor.
#[cfg(unix)]
fn read_exact_at(file: &std::fs::File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;

    file.read_exact_at(buf, offset)
}

/// Reads exactly `buf.len()` bytes from `file` starting at `offset` without changing a shared cursor.
#[cfg(windows)]
fn read_exact_at(file: &std::fs::File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;

    while !buf.is_empty() {
        let bytes_read = file.seek_read(buf, offset)?;
        if bytes_read == 0 {
            return Err(std::io::Error::from(ErrorKind::UnexpectedEof));
        }

        buf = &mut buf[bytes_read..];
        offset += bytes_read as u64;
    }

    Ok(())
}
