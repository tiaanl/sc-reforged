use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, SeekFrom},
    path::{Path, PathBuf},
    rc::Rc,
};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::engine::utils;

#[derive(Debug, thiserror::Error)]
pub enum FileSystemError {
    /// This error is generated if the path we requested could not be found, either as an external
    /// file or a file inside a .gut archive.
    #[error("Path not found: {0}")]
    NotFound(PathBuf),

    #[error("A .gut file ({0}) does not exist for the specified path: {1}")]
    GutFileNotFound(PathBuf, PathBuf),

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
enum PathPointer {
    External(PathBuf),
    Internal(PathBuf, PathBuf),
}

pub struct VirtualFileSystem {
    root_path: PathBuf,
    guts: RefCell<HashMap<PathBuf, Rc<GutFile>>>,
}

impl VirtualFileSystem {
    pub fn new(root_path: impl AsRef<Path>) -> std::io::Result<Self> {
        let root_path = root_path.as_ref().canonicalize()?;

        Ok(Self {
            root_path,
            guts: RefCell::new(HashMap::default()),
        })
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        // Check if the external file exists.
        let external_path = self.root_path.join(&path);
        if external_path.exists() {
            return Ok(std::fs::read(external_path)?);
        }

        let gut_path = self.gut_path_for(&path);
        if !gut_path.exists() {
            return Err(FileSystemError::GutFileNotFound(
                gut_path,
                path.as_ref().to_owned(),
            ));
        }

        let gut_file = self.get_gut_file(&gut_path)?;
        if !gut_file.path_exists(&path) {
            return Err(FileSystemError::GutFileNotFound(
                gut_path,
                path.as_ref().to_owned(),
            ));
        }

        gut_file.get_contents(path)
    }

    fn get_gut_file(&self, gut_path: &PathBuf) -> Result<Rc<GutFile>, FileSystemError> {
        if let Some(gut_file) = self.guts.borrow().get(gut_path) {
            return Ok(Rc::clone(gut_file));
        }

        // Try to load the .gut file.
        let gut_file = Rc::new(match GutFile::from_file(gut_path) {
            Ok(gut_file) => gut_file,
            Err(err) => {
                // TODO: Put some entry in the guts cache that tells us not to retry creating .gut
                //       files that could not be loaded previously.
                tracing::warn!(
                    "Could not open .gut file - {} - {}",
                    gut_path.display(),
                    err
                );
                return Err(FileSystemError::Io(err));
            }
        });

        self.guts
            .borrow_mut()
            .insert(gut_path.clone(), Rc::clone(&gut_file));

        Ok(gut_file)
    }

    pub fn enum_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, std::io::Error> {
        let gut_path = self
            .root_path
            .join(path.as_ref().components().next().unwrap())
            .with_extension("gut");

        let mut entries = vec![];

        if let Some(gut) = self.guts.borrow().get(&gut_path) {
            // let path = GutFile::to_gut_file_path(&path);
            // let path = path.to_string_lossy();
            // Use string comparison here, becuase PathBuf::starts_woth uses path seperators which
            // might not be the same as we need for .gut files.
            gut.entries
                .values()
                .filter(|e| e.name.starts_with(&path))
                .for_each(|e| entries.push(e.name.clone()));
        }

        // Add external files.
        let search_path = self.root_path.join(path);

        walkdir::WalkDir::new(search_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .for_each(|e| {
                // We unwrap here, because we got all the paths from the filesystem, so we're sure
                // they exist and their paths should match.
                let p = pathdiff::diff_paths(e.path(), &self.root_path).unwrap();
                entries.push(p);
            });

        Ok(entries)
    }

    fn gut_path_for(&self, path: impl AsRef<Path>) -> PathBuf {
        let first = path.as_ref().components().next().unwrap();
        self.root_path.join(first).with_extension("gut")
    }
}

#[derive(Debug)]
struct Entry {
    name: PathBuf,
    offset: u32,
    size: u32,
    is_plain_text: bool,
    name_hash: u32,
}

struct GutFile {
    path: PathBuf,
    entries: HashMap<PathBuf, Entry>,
}

impl GutFile {
    fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
        use std::io::{Read, Seek};

        let mut reader = std::fs::File::open(path.as_ref())?;

        utils::skip_sinister_header(&mut reader)?;

        let _hash_1 = reader.read_u32::<LE>()?;
        let _hash_2 = reader.read_u32::<LE>()?;

        let entry_count = reader.read_u32::<LE>()?;
        let mut filename: [u8; 32] = [0; 32];
        reader.read_exact(&mut filename)?;

        let header_size = reader.stream_position()? as u32;

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            entries: Self::read_entries(&mut reader, entry_count, header_size)?,
        })
    }

    pub fn path_exists(&self, path: impl AsRef<Path>) -> bool {
        let path = PathBuf::from(path.as_ref().to_string_lossy().to_lowercase());
        self.entries.contains_key(&path)
    }

    fn read_entries<R>(
        reader: &mut R,
        entry_count: u32,
        header_size: u32,
    ) -> Result<HashMap<PathBuf, Entry>, std::io::Error>
    where
        R: std::io::Read,
    {
        let mut entries = HashMap::default();
        for _ in 0..entry_count {
            let name_length = reader.read_u32::<LE>()?;
            let size = reader.read_u32::<LE>()?;
            let offset = reader.read_u32::<LE>()?;
            let is_plain_text = reader.read_u32::<LE>()? != 0;
            let name_hash = reader.read_u32::<LE>()?;

            // Read variable length string.
            let mut name = vec![0; name_length as usize - 1];
            reader.read_exact(&mut name)?;
            utils::crypt(&mut name);

            // Read the null terminator after the name.
            let _ = reader.read_u8()?;

            // All paths inside the .gut file are in all lowercase, but also change the path
            // separators to use the OS main separator to make it easier to work `std::path::*`.
            let name = PathBuf::from(
                std::str::from_utf8(&name)
                    .unwrap()
                    .chars()
                    .map(|c| {
                        if c == '\\' {
                            std::path::MAIN_SEPARATOR
                        } else {
                            c
                        }
                    })
                    .collect::<String>(),
            );

            entries.insert(
                name.clone(),
                Entry {
                    name,
                    offset: header_size + offset,
                    size,
                    is_plain_text,
                    name_hash,
                },
            );
        }
        Ok(entries)
    }

    fn get_contents(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        use std::io::Seek;

        let path = Self::normalize_path(path);

        let Some(entry) = self.entries.get(&path) else {
            return Err(FileSystemError::NotFound(path));
        };

        let mut reader = std::fs::File::open(&self.path)?;
        reader.seek(SeekFrom::Start(entry.offset as u64))?;

        let mut buf = vec![0; entry.size as usize];
        reader.read_exact(&mut buf)?;

        if entry.is_plain_text {
            utils::crypt(&mut buf);
        }

        Ok(buf)
    }

    fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
        use std::path::Component as C;

        let mut normalized = PathBuf::new();

        for component in path.as_ref().components() {
            match component {
                C::RootDir => normalized.push(component),
                C::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                C::Normal(part) => {
                    let part = part.to_string_lossy().to_lowercase();
                    normalized.push(part);
                }
                _ => {}
            }
        }

        normalized
    }
}
