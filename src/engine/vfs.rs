use std::{
    io::{Read, SeekFrom},
    path::{Path, PathBuf},
};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::engine::utils;

#[derive(Debug, thiserror::Error)]
pub enum VirtualFileSystemError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct VirtualFileSystem {
    root_path: PathBuf,
}

impl VirtualFileSystem {
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            root_path: root_path.as_ref().to_owned(),
        }
    }

    pub fn open(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, VirtualFileSystemError> {
        let external_path = self.root_path.join(path.as_ref());
        if external_path.exists() {
            let mut file = std::fs::File::open(external_path)?;
            let file_size = file.metadata()?.len();
            let mut buf = vec![0; file_size as usize];
            file.read_exact(&mut buf)?;
            return Ok(buf);
        }

        if let Some(gut_file_path) = self.find_gut_file_path_for(path.as_ref()) {
            let gut_file = GutFile::from_file(gut_file_path)?;
            let buf = gut_file.get_contents(path.as_ref())?;
            return Ok(buf);
        }

        let err: std::io::Error = std::io::ErrorKind::NotFound.into();
        Err(err.into())
    }

    fn find_gut_file_path_for(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref().to_path_buf();
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
    entries: Vec<Entry>,
}

impl GutFile {
    fn from_file(path: impl AsRef<Path>) -> std::io::Result<Self> {
        use std::io::{Read, Seek};

        let mut r = std::fs::File::open(path.as_ref())?;

        utils::skip_sinister_header(&mut r)?;

        let _hash_1 = r.read_u32::<LittleEndian>()?;
        let _hash_2 = r.read_u32::<LittleEndian>()?;

        let entry_count = r.read_u32::<LittleEndian>()?;
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
    ) -> Result<Vec<Entry>, std::io::Error> {
        let mut entries = vec![];
        for _ in 0..entry_count {
            let name_length = r.read_u32::<LittleEndian>()?;
            let size = r.read_u32::<LittleEndian>()?;
            let offset = r.read_u32::<LittleEndian>()?;
            let is_plain_text = r.read_u32::<LittleEndian>()? != 0;
            let name_hash = r.read_u32::<LittleEndian>()?;

            // Read variable length string.
            let mut name = vec![0; name_length as usize - 1];
            r.read_exact(&mut name)?;
            utils::crypt(&mut name);
            let name = String::from_utf8_lossy(&name).to_string();
            // Read the null terminator after the name.
            r.read_u8()?;

            entries.push(Entry {
                name,
                offset: header_size + offset,
                size,
                is_plain_text,
                _name_hash: name_hash,
            })
        }
        Ok(entries)
    }

    fn get_contents(&self, path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        use std::io::Seek;

        // The entries in the .gut file uses "\".
        let path = path.as_ref().to_string_lossy().replace("/", "\\");

        let entry = self.entries.iter().find(|e| e.name == path);

        let Some(entry) = entry else {
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
