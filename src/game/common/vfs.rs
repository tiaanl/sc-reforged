use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    io::{Read, SeekFrom},
    path::{Path, PathBuf},
    rc::Rc,
};

use byteorder::{LittleEndian as LE, ReadBytesExt};

use crate::engine::utils;

#[derive(Debug, thiserror::Error)]
pub enum FileSystemError {
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
pub enum VirtualPath {
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

    pub fn path_for(&self, path: impl AsRef<Path>) -> Option<VirtualPath> {
        debug_assert!(
            !path.as_ref().is_absolute(),
            "Can't use absolute paths in the virtual file system."
        );

        // Check if the external file exists.
        let external_path = change_separator(
            self.root_path.join(path.as_ref()),
            std::path::MAIN_SEPARATOR,
        );
        if external_path.exists() {
            return Some(VirtualPath::External(external_path));
        }

        let first = path.as_ref().components().next().unwrap();
        let gut_path = self.root_path.join(first).with_extension("gut");
        if gut_path.exists() {
            Some(VirtualPath::Internal(gut_path, path.as_ref().to_path_buf()))
        } else {
            None
        }
    }

    // This one is cool, but too complex and slow.
    pub fn _path_for(&self, path: impl AsRef<Path>) -> Option<VirtualPath> {
        debug_assert!(
            !path.as_ref().is_absolute(),
            "Can't use absolute paths in the virtual file system."
        );

        // Check if the external file exists.
        let external_path = change_separator(
            self.root_path.join(path.as_ref()),
            std::path::MAIN_SEPARATOR,
        );
        if external_path.exists() {
            return Some(VirtualPath::External(external_path));
        }

        let mut path_parts = VecDeque::default();

        // Try to find a .gut file in the path.
        let mut gut_parts = path
            .as_ref()
            .components()
            .filter_map(|component| {
                if let std::path::Component::Normal(part) = component {
                    Some(PathBuf::from(part))
                } else {
                    // We just ignore the other component types for now.
                    None
                }
            })
            .collect::<Vec<_>>();

        let has_extension = gut_parts
            .last()
            .and_then(|l| l.extension())
            .map(|e| !e.is_empty())
            .unwrap_or(false);

        // If the last part has an extension, then we assume it's a file.
        if has_extension {
            path_parts.push_front(gut_parts.pop().unwrap());
        }

        while !gut_parts.is_empty() {
            // Build a path to the .gut file.
            let gut_path = gut_parts
                .iter()
                .fold(self.root_path.clone(), |i, p| i.join(p))
                .with_extension("gut");

            if gut_path.exists() {
                // Because the internal paths start with the name of the .gut, we have to prepend
                // it to our path parts.
                path_parts.push_front(gut_parts.last().unwrap().clone());

                // Assemble the internal path.
                let path = path_parts
                    .into_iter()
                    .fold(PathBuf::new(), |i, p| i.join(p));

                return Some(VirtualPath::Internal(
                    gut_path,
                    change_separator(path, '\\'),
                ));
            }

            // We can unwrap, because we checked in the while loop if we're empty already.
            path_parts.push_front(gut_parts.pop().unwrap());
        }

        None
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, FileSystemError> {
        let Some(path) = self.path_for(&path) else {
            return Err(FileSystemError::Io(std::io::ErrorKind::NotFound.into()));
        };

        match path {
            VirtualPath::External(path) => return Ok(std::fs::read(path)?),
            VirtualPath::Internal(gut_path, path) => {
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
            .join(path.as_ref().components().next().unwrap())
            .with_extension("gut");

        let mut entries = vec![];

        if let Some(gut) = self.guts.borrow().get(&gut_path) {
            gut.entries
                .keys()
                .filter(|e| e.starts_with(&path))
                .for_each(|e| entries.push(e.clone()));
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
    name: String,
    offset: u32,
    size: u32,
    is_plain_text: bool,
    _name_hash: u32,
}

struct GutFile {
    path: PathBuf,
    entries: HashMap<PathBuf, Entry>,
}

impl GutFile {
    fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
        use std::io::{Read, Seek};

        let mut r = std::fs::File::open(path.as_ref())?;

        utils::skip_sinister_header(&mut r)?;

        let _hash_1 = r.read_u32::<LE>()?;
        let _hash_2 = r.read_u32::<LE>()?;

        let entry_count = r.read_u32::<LE>()?;
        let mut filename: [u8; 32] = [0; 32];
        r.read_exact(&mut filename)?;

        let header_size = r.stream_position()? as u32;

        Ok(Self {
            path: path.as_ref().to_path_buf(),
            entries: Self::read_entries(r, entry_count, header_size)?,
        })
    }

    fn read_entries(
        mut r: impl std::io::Read,
        entry_count: u32,
        header_size: u32,
    ) -> Result<HashMap<PathBuf, Entry>, std::io::Error> {
        let mut entries = HashMap::default();
        for _ in 0..entry_count {
            let name_length = r.read_u32::<LE>()?;
            let size = r.read_u32::<LE>()?;
            let offset = r.read_u32::<LE>()?;
            let is_plain_text = r.read_u32::<LE>()? != 0;
            let name_hash = r.read_u32::<LE>()?;

            // Read variable length string.
            let mut name = vec![0; name_length as usize - 1];
            r.read_exact(&mut name)?;
            utils::crypt(&mut name);
            // Store the names in all lowercase for comparisons.
            let name = String::from_utf8_lossy(&name)
                .to_ascii_lowercase()
                .to_string();
            // Read the null terminator after the name.
            r.read_u8()?;

            entries.insert(
                PathBuf::from(&name),
                Entry {
                    name,
                    offset: header_size + offset,
                    size,
                    is_plain_text,
                    _name_hash: name_hash,
                },
            );
        }
        Ok(entries)
    }

    fn get_contents(&self, path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        use std::io::Seek;

        // The entries in the .gut file uses "\".
        // Do a case-insensitive camparison.
        let path = change_separator(path, '\\');

        let Some(entry) = self.entries.get(&path) else {
            return Err(std::io::ErrorKind::NotFound.into());
        };

        let mut r = std::fs::File::open(&self.path)?;
        if r.seek(SeekFrom::Start(entry.offset as u64))? != entry.offset as u64 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        let mut buf = vec![0; entry.size as usize];
        r.read_exact(&mut buf)?;

        if entry.is_plain_text {
            utils::crypt(&mut buf);
        }

        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let fs = VirtualFileSystem::new(r"C:\games\shadow_company\data-pristine").unwrap();

        let p = fs.path_for(r"config\lod_model_profiles\palm1_lod_model.txt");

        println!("{:?}", p);
    }
}
