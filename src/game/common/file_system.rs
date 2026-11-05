use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use ahash::HashMap;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("The game data directory could not be found. ({0})")]
    RootDirNotFound(PathBuf),

    #[error("Invalid GUT entry. ({0})")]
    InvalidGutEntry(PathBuf),

    #[error("The path was not found. ({0})")]
    FileNotFound(PathBuf),

    #[error("I/O Error. ({0})")]
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

        for entry in WalkDir::new(&root_dir).into_iter() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(error = %err, "Could not scan data directory.");
                    continue;
                }
            };

            let path = entry.path();

            if !entry.file_type().is_file()
                || !path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("gut"))
            {
                continue;
            }

            match GutFile::from_path(path) {
                Ok(gut_file) => {
                    if let Some(name) = path.file_stem() {
                        gut_files.insert(name.to_string_lossy().to_ascii_lowercase(), gut_file);
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %err,
                        "Could not load GUT archive.",
                    );
                }
            }
        }

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

    /// Returns all files below `root`, combining external files and matching .gut entries. External
    /// files take precedence over archive entries with the same case-insensitive path.
    pub fn dir(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<impl Iterator<Item = PathBuf>, FileSystemError> {
        let root = root.as_ref();
        debug_assert!(
            root.components()
                .any(|component| matches!(component, Component::Normal(_))),
            "The directory path must contain at least one named component. ({})",
            root.display(),
        );

        let mut result: HashMap<String, PathBuf> = HashMap::default();

        if let Some(gut_file) = self.gut_file_for_path(root) {
            let search = normalize_gut_path(root).trim_end_matches('\\').to_owned();
            let prefix = format!("{search}\\");

            for gut_path in gut_file.entries.keys() {
                if gut_path.starts_with(&prefix) {
                    // Convert the separators to OS separators again.
                    result.insert(
                        gut_path.clone(),
                        PathBuf::from(gut_path.replace(r"\", std::path::MAIN_SEPARATOR_STR)),
                    );
                }
            }
        }

        let root = self.root_dir.join(root);

        for entry in WalkDir::new(&root) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(
                        path = %root.display(),
                        error = %err,
                        "Could not enumerate directory.",
                    );
                    continue;
                }
            };

            if entry.file_type().is_file()
                && let Ok(path) = entry.path().strip_prefix(&self.root_dir)
            {
                // Keep the disk spelling so the returned override also loads on case-sensitive
                // hosts.
                result.insert(normalize_gut_path(path), path.to_path_buf());
            }
        }

        Ok(result.into_values())
    }

    fn gut_file_for_path(&self, path: impl AsRef<Path>) -> Option<&GutFile> {
        let component = path.as_ref().components().next()?;
        let name = component.as_os_str().to_str()?.to_ascii_lowercase();

        self.gut_files.get(&name)
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
        let file_len = file.metadata()?.len();

        let mut entries: HashMap<String, GutEntry> = HashMap::default();

        for entry in gut_file.entries() {
            let end = entry
                .offset
                .checked_add(entry.size)
                .ok_or(FileSystemError::InvalidGutEntry(PathBuf::from(&entry.name)))?;

            if end > file_len || usize::try_from(entry.size).is_err() {
                return Err(FileSystemError::InvalidGutEntry(PathBuf::from(&entry.name)));
            }

            entries.insert(
                entry.name.to_ascii_lowercase(),
                GutEntry {
                    offset: entry.offset,
                    size: entry.size,
                    is_plain_text: entry.is_plain_text,
                },
            );
        }

        Ok(Self { file, entries })
    }

    /// Reads a single file entry.
    fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        // Paths in a .gut file are all lower case and use the `\`` separator.
        let lookup = normalize_gut_path(path.as_ref());

        let Some(entry) = self.entries.get(&lookup) else {
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
