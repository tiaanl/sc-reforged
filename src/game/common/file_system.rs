use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use ahash::{HashMap, HashSet};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("The game data directory could not be found. \"{0}\"")]
    RootDirNotFound(PathBuf),

    #[error("The path was not found: {0}")]
    FileNotFound(PathBuf),

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct FileSystem {
    root_dir: PathBuf,
    gut_files: HashMap<String, GutFile>,
}

impl FileSystem {
    /// Builds a virtual file system rooted at the game data directory.
    pub fn new(root_dir: impl AsRef<Path>) -> Result<Self, FileSystemError> {
        let root_dir = root_dir.as_ref().canonicalize()?;

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

        Ok(Self {
            root_dir,
            gut_files,
        })
    }

    /// Loads a file either from disk or from a mounted .gut archive.
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        let path = path.as_ref();
        let full_path = self.root_dir.join(path);

        match std::fs::read(full_path) {
            Ok(data) => Ok(data),

            Err(err) if err.kind() == ErrorKind::NotFound => {
                if let Some(gut_file) = self.gut_file_for_path(path) {
                    gut_file.load(path)
                } else {
                    Err(FileSystemError::FileNotFound(path.to_path_buf()))
                }
            }

            Err(err) => Err(err.into()),
        }
    }

    /// Returns all files below `root`, combining external files and matching .gut entries.
    pub fn dir(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<impl Iterator<Item = PathBuf>, FileSystemError> {
        // TODO: Enumerate the root directory?
        let mut result: HashSet<PathBuf> = HashSet::default();

        if let Some(gut_file) = self.gut_file_for_path(&root) {
            let search = normalize_gut_path(root.as_ref())
                .trim_end_matches('\\')
                .to_owned();
            let prefix = format!("{search}\\");

            for gut_path in gut_file.entries.keys() {
                if gut_path.starts_with(&prefix) {
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

struct GutFile {
    file: std::fs::File,
    entries: HashMap<String, GutEntry>,
}

impl GutFile {
    fn from_path(path: impl AsRef<Path>) -> Result<Self, FileSystemError> {
        use shadow_company_tools::gut;

        let mut file = std::fs::File::open(path.as_ref())?;
        let gut_file = gut::GutFile::open(&mut file).map_err(|gut::GutError::Io(err)| err)?;

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
        let lookup = normalize_gut_path(path.as_ref()).to_string();

        let Some(entry) = self.entries.get(&lookup) else {
            return Err(FileSystemError::FileNotFound(path.as_ref().to_path_buf()));
        };

        let size = usize::try_from(entry.size)
            .map_err(|_| std::io::Error::from(ErrorKind::InvalidData))?;

        let mut data = vec![0u8; size];
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

fn normalize_gut_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

fn read_exact_at(file: &std::fs::File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    let mut buf = buf;
    let mut offset = offset;

    while !buf.is_empty() {
        let bytes_read = loop {
            #[cfg(unix)]
            let result = {
                use std::os::unix::fs::FileExt;
                file.read_at(buf, offset)
            };

            #[cfg(windows)]
            let result = {
                use std::os::windows::fs::FileExt;
                file.seek_read(buf, offset)
            };

            match result {
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                result => break result?,
            }
        };

        if bytes_read == 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }

        buf = &mut buf[bytes_read..];

        if !buf.is_empty() {
            offset = offset
                .checked_add(bytes_read as u64)
                .ok_or(ErrorKind::InvalidInput)?;
        }
    }

    Ok(())
}
