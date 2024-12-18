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

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
}

fn change_separator(path: impl AsRef<Path>, separator: char) -> PathBuf {
    PathBuf::from(
        path.as_ref()
            .to_string_lossy()
            .chars()
            .map(|c| if c == '/' || c == '\\' { separator } else { c })
            .collect::<String>(),
    )
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
        // Make sure the root_path uses the OS separators.
        let root_path = change_separator(root_path, std::path::MAIN_SEPARATOR);

        if !root_path.exists() {
            return Err(std::io::ErrorKind::NotFound.into());
        }

        Ok(Self {
            root_path,
            guts: RefCell::new(HashMap::default()),
        })
    }

    /// Find a way to get to the specified `path`.
    fn path_for(&self, path: impl AsRef<Path>) -> Option<PathPointer> {
        // Check if the external file exists.
        let external_path = self.root_path.join(path.as_ref().to_path_buf());
        if external_path.exists() {
            return Some(PathPointer::External(external_path));
        }

        let first = path.as_ref().components().next().unwrap();
        let gut_path = self.root_path.join(first).with_extension("gut");

        if !gut_path.exists() {
            return None;
        }

        // Load the .gut file and check if the path exists inside.
        let gut_file = self.get_gut_file(&gut_path)?;
        if gut_file.path_exists(&path) {
            Some(PathPointer::Internal(gut_path, path.as_ref().to_path_buf()))
        } else {
            None
        }
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        let Some(path) = self.path_for(&path) else {
            return Err(FileSystemError::NotFound(path.as_ref().to_path_buf()));
        };

        match path {
            PathPointer::External(path) => return Ok(std::fs::read(path)?),
            PathPointer::Internal(gut_path, path) => {
                let Some(gut_file) = self.get_gut_file(&gut_path) else {
                    return Err(FileSystemError::Io(std::io::ErrorKind::NotFound.into()));
                };

                gut_file.get_contents(path).map_err(FileSystemError::Io)
            }
        }
    }

    fn get_gut_file(&self, gut_path: &PathBuf) -> Option<Rc<GutFile>> {
        if let Some(gut_file) = self.guts.borrow().get(gut_path) {
            return Some(Rc::clone(gut_file));
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
                return None;
            }
        });

        self.guts
            .borrow_mut()
            .insert(gut_path.clone(), Rc::clone(&gut_file));

        Some(gut_file)
    }

    pub fn enum_dir(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, std::io::Error> {
        let gut_path = self
            .root_path
            .join(path.as_ref().to_path_buf().components().next().unwrap())
            .with_extension("gut");

        let mut entries = vec![];

        if let Some(gut) = self.guts.borrow().get(&gut_path) {
            gut.entries
                .values()
                .filter(|e| e.name.starts_with(&path))
                .for_each(|e| entries.push(e.name.clone()));
        }

        // Add external files.
        let search_path = self
            .root_path
            .join(change_separator(path, std::path::MAIN_SEPARATOR));

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

    fn find_gut_file_path_for(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        // Use OS separators for the path, because we'll be checking the filesystem with it.
        let path = change_separator(path, std::path::MAIN_SEPARATOR);

        let first = path.components().next()?;
        let path = self.root_path.join(first).with_extension("gut");

        if path.exists() {
            Some(path)
        } else {
            None
        }
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
            let name = PathBuf::from(std::str::from_utf8(&name).unwrap());

            // Read the null terminator after the name.
            let _ = reader.read_u8()?;

            entries.insert(
                PathBuf::from(&name),
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

    fn get_contents(&self, path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        use std::io::Seek;

        // The entries in the .gut file uses "\".
        // Do a case-insensitive camparison.
        let path = PathBuf::from(
            change_separator(path, '\\')
                .to_string_lossy()
                .to_ascii_lowercase(),
        );

        let entry = self
            .entries
            .get(&path)
            .expect("The entry should exist, because we checked for it in path_for!");

        let mut reader = std::fs::File::open(&self.path)?;
        reader.seek(SeekFrom::Start(entry.offset as u64))?;

        let mut buf = vec![0; entry.size as usize];
        reader.read_exact(&mut buf)?;

        if entry.is_plain_text {
            utils::crypt(&mut buf);
        }

        Ok(buf)
    }
}
