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
        let root_dir = to_native_path(root_dir.as_ref()).canonicalize()?;
        if !root_dir.is_dir() {
            return Err(FileSystemError::RootDirNotFound(root_dir));
        }

        let mut gut_files = HashMap::default();

        for entry in WalkDir::new(&root_dir).into_iter() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(error = %err, "Could not scan data directory.");
                    continue;
                }
            };

            let gut_native_path = entry.path();

            if !entry.file_type().is_file()
                || !gut_native_path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("gut"))
            {
                continue;
            }

            match GutFile::from_path(gut_native_path) {
                Ok(gut_file) => {
                    if let Some(name) = gut_native_path.file_stem() {
                        gut_files.insert(name.to_string_lossy().to_ascii_lowercase(), gut_file);
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        path = %gut_native_path.display(),
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

    /// Loads a path relative to the game data directory, from disk or a mounted .gut archive.
    /// Accepts either slash style, converting to a native path for disk access or a virtual path
    /// for archive lookup. Native paths preserve case and virtual paths use lowercase ASCII.
    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        let path = path.as_ref();
        let native_path = self.root_dir.join(to_native_path(path));

        match std::fs::read(native_path) {
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
    /// `root` is a path relative to the game data directory and accepts either slash style.
    /// Returned paths use native separators and preserve disk filename case.
    pub fn dir(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<impl Iterator<Item = PathBuf>, FileSystemError> {
        let root = root.as_ref();
        let native_root = to_native_path(root);
        debug_assert!(
            native_root
                .components()
                .any(|component| matches!(component, Component::Normal(_))),
            "The directory path must contain at least one named component. ({})",
            root.display(),
        );

        let mut result: HashMap<String, PathBuf> = HashMap::default();

        if let Some(gut_file) = self.gut_file_for_path(root) {
            let search = to_virtual_path(root).trim_end_matches('\\').to_owned();
            let prefix = format!("{search}\\");

            for virtual_path in gut_file.entries.keys() {
                if virtual_path.starts_with(&prefix) {
                    // Convert the virtual path to native separators for callers.
                    result.insert(
                        virtual_path.clone(),
                        to_native_path(Path::new(virtual_path)),
                    );
                }
            }
        }

        let native_root = self.root_dir.join(native_root);

        for entry in WalkDir::new(&native_root) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    tracing::warn!(
                        path = %native_root.display(),
                        error = %err,
                        "Could not enumerate directory.",
                    );
                    continue;
                }
            };

            if entry.file_type().is_file()
                && let Ok(native_path) = entry.path().strip_prefix(&self.root_dir)
            {
                // Keep the disk spelling so the returned override also loads on case-sensitive
                // hosts.
                result.insert(to_virtual_path(native_path), native_path.to_path_buf());
            }
        }

        Ok(result.into_values())
    }

    fn gut_file_for_path(&self, path: impl AsRef<Path>) -> Option<&GutFile> {
        let virtual_path = to_virtual_path(path.as_ref());
        self.gut_files.get(virtual_path.split('\\').next()?)
    }
}

struct GutFile {
    file: std::fs::File,
    entries: HashMap<String, GutEntry>,
}

impl GutFile {
    /// Opens an archive at a native filesystem path and indexes its virtual paths.
    fn from_path(gut_native_path: impl AsRef<Path>) -> Result<Self, FileSystemError> {
        use shadow_company_tools::gut;

        let mut file = std::fs::File::open(gut_native_path.as_ref())?;
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

    /// Reads an entry using the virtual representation of a relative path.
    fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        let virtual_path = to_virtual_path(path.as_ref());

        let Some(entry) = self.entries.get(&virtual_path) else {
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

/// Converts a path using either slash style to native separators, preserving case.
/// Removes redundant separators and `.` components without accessing the filesystem.
fn to_native_path(path: &Path) -> PathBuf {
    let native_path = path
        .to_string_lossy()
        .replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR);
    Path::new(&native_path)
        .components()
        .filter(|component| !matches!(component, Component::CurDir))
        .collect()
}

/// Converts a path using either slash style to a virtual path for GUT archive lookup.
/// Virtual paths use backslash separators and lowercase ASCII, independent of the host filesystem.
fn to_virtual_path(path: &Path) -> String {
    to_native_path(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("\\")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        /// Creates an isolated data directory for a filesystem test.
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "sc-reforged-file-system-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed),
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn new_requires_a_directory() {
        let root = TestDirectory::new();
        let file = root.0.join("data.txt");
        std::fs::write(&file, b"data").unwrap();

        assert!(FileSystem::new(&root.0).is_ok());
        assert!(matches!(
            FileSystem::new(&file),
            Err(FileSystemError::RootDirNotFound(path)) if path == file
        ));
    }

    #[test]
    fn equivalent_paths_convert_to_the_same_virtual_path() {
        for path in [
            "config/profile.txt",
            "./CONFIG//profile.txt",
            "config/./profile.txt",
            r".\config\\.\profile.txt",
        ] {
            assert_eq!(to_virtual_path(Path::new(path)), r"config\profile.txt");
        }
    }

    #[test]
    fn loose_paths_accept_both_separators_and_follow_native_case_rules() {
        let root = TestDirectory::new();
        std::fs::create_dir_all(root.0.join("Config/Profiles")).unwrap();
        std::fs::write(root.0.join("Config/Profiles/MiXeD.TXT"), b"loose").unwrap();
        let fs = FileSystem::new(&root.0).unwrap();
        let expected = PathBuf::from("Config").join("Profiles").join("MiXeD.TXT");

        for directory in [
            "Config/Profiles",
            r"Config\Profiles",
            r".\Config//Profiles/.",
        ] {
            let paths: Vec<_> = fs.dir(directory).unwrap().collect();
            assert_eq!(paths, vec![expected.clone()]);
            assert_eq!(fs.load(&paths[0]).unwrap(), b"loose");
            for separator in ["/", "\\"] {
                let path = format!("{directory}{separator}MiXeD.TXT");
                assert_eq!(fs.load(path).unwrap(), b"loose");
            }
        }

        // The volume decides whether alternate casing names the same loose file.
        assert_eq!(
            fs.load(r"Config\Profiles\mixed.txt").ok(),
            std::fs::read(root.0.join("Config/Profiles/mixed.txt")).ok(),
        );
        for separator in ["/", "\\"] {
            let data_path = root.0.to_str().unwrap().replace(['/', '\\'], separator);
            assert!(FileSystem::new(data_path).is_ok());
        }
    }

    #[test]
    fn archive_selection_listing_and_loading_use_normalized_paths() {
        let root = TestDirectory::new();
        let gut_native_path = root.0.join("archive.bin");
        std::fs::write(&gut_native_path, b"archived").unwrap();
        let mut fs = FileSystem::new(&root.0).unwrap();
        fs.gut_files.insert(
            "config".to_owned(),
            GutFile {
                file: std::fs::File::open(gut_native_path).unwrap(),
                entries: [(
                    r"config\profiles\profile.txt".to_owned(),
                    GutEntry {
                        offset: 0,
                        size: 8,
                        is_plain_text: false,
                    },
                )]
                .into_iter()
                .collect(),
            },
        );
        let expected = PathBuf::from("config").join("profiles").join("profile.txt");

        for directory in [
            "config/profiles",
            "./config//profiles/",
            "config/./profiles",
            r".\config\\profiles",
            r".\CONFIG/PROFILES",
        ] {
            assert_eq!(
                fs.dir(directory).unwrap().collect::<Vec<_>>(),
                vec![expected.clone()],
            );
            assert_eq!(
                fs.load(format!("{directory}/profile.txt")).unwrap(),
                b"archived",
            );
        }

        // Equivalent disk paths must replace the archive entry while preserving disk casing.
        std::fs::create_dir_all(root.0.join("config/profiles")).unwrap();
        std::fs::write(root.0.join("config/profiles/Profile.TXT"), b"override").unwrap();
        let paths: Vec<_> = fs.dir("./config//profiles").unwrap().collect();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].file_name().unwrap(), "Profile.TXT");
        assert_eq!(fs.load(&paths[0]).unwrap(), b"override");
    }
}
