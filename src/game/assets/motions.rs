use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use shadow_company_tools::bmf;

use crate::{
    engine::{
        assets::AssetError,
        storage::{Handle, StorageMap},
    },
    game::{
        assets::motion::{Motion, MotionFlags, State},
        file_system::FileSystem,
    },
};

pub struct Motions {
    file_system: Arc<FileSystem>,
    storage: RwLock<StorageMap<String, Motion, Arc<Motion>>>,
}

impl Motions {
    pub fn new(file_system: Arc<FileSystem>) -> Self {
        Self {
            file_system,
            storage: RwLock::new(StorageMap::default()),
        }
    }

    pub fn get(&self, handle: Handle<Motion>) -> Option<Arc<Motion>> {
        self.storage.read().unwrap().get(handle).map(Arc::clone)
    }

    pub fn get_by_key(&self, name: &str) -> Option<Arc<Motion>> {
        self.storage
            .read()
            .unwrap()
            .get_by_key(&name.to_string())
            .map(Arc::clone)
    }

    pub fn load(&self, name: impl Into<String>) -> Result<Handle<Motion>, AssetError> {
        let name = name.into();

        {
            let storage = self.storage.read().unwrap();
            if let Some(handle) = storage.get_handle_by_key(&name) {
                return Ok(handle);
            }
        }

        let path = PathBuf::from("motions").join(&name).with_extension("bmf");
        let data = self.file_system.load(&path)?;
        let motion = build_motion_from_memory(&path, &data)?;

        let handle = {
            let mut storage = self.storage.write().unwrap();
            storage.insert(name, Arc::new(motion))
        };

        Ok(handle)
    }
}

fn build_motion_from_memory(path: &PathBuf, data: &[u8]) -> Result<Motion, AssetError> {
    use std::sync::atomic::AtomicU32;

    const MOTION_HEADER_RUNTIME_TICKS_OFFSET: usize = 0x9c;

    fn read_u32_le_at(data: &[u8], offset: usize) -> Option<u32> {
        let bytes = data.get(offset..offset + 4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    let bmf = bmf::Motion::read(&mut std::io::Cursor::new(data))
        .map_err(|err| AssetError::from_io_error(err, path.as_ref()))?;

    let runtime_ticks_per_frame =
        read_u32_le_at(data, MOTION_HEADER_RUNTIME_TICKS_OFFSET).unwrap_or(bmf.ticks_per_frame);

    Ok(Motion {
        name: bmf.name,
        frame_count: bmf.key_frame_count,
        last_frame: bmf.last_frame,
        base_ticks_per_frame: runtime_ticks_per_frame.max(1),
        from_state: State::from_motion_state_id(bmf.from_state),
        to_state: State::from_motion_state_id(bmf.to_state),
        key_frames: bmf.key_frames,
        flags: AtomicU32::new(MotionFlags::empty().bits()),
    })
}
