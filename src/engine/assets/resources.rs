use std::{
    any::Any,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use parking_lot::RwLock;

use crate::game::file_system as fs;

pub struct ResourceLoadContext {
    resources: Resources,
    path: PathBuf,
}

impl ResourceLoadContext {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load_direct(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, ()> {
        self.resources
            .0
            .file_system
            .load(path.as_ref())
            .map_err(|_| ())
    }
}

pub trait ResourceType: Sized {
    fn from_data(data: Vec<u8>, context: &ResourceLoadContext) -> Result<Self, ()>;
}

pub struct Resource<R: ResourceType> {
    resources: Resources,
    data: Arc<R>,
}

impl<R: ResourceType> std::ops::Deref for Resource<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<R: ResourceType> Clone for Resource<R> {
    fn clone(&self) -> Self {
        Self {
            resources: self.resources.clone(),
            data: Arc::clone(&self.data),
        }
    }
}

#[derive(Clone, bevy_ecs::resource::Resource)]
pub struct Resources(Arc<ResourcesInner>);

impl Resources {
    pub fn new(data_dir: impl AsRef<Path>) -> Result<Self, ()> {
        Ok(Self(Arc::new(ResourcesInner::new(data_dir)?)))
    }

    pub fn request<R>(&self, path: impl AsRef<Path>) -> Result<Resource<R>, ()>
    where
        R: ResourceType + 'static,
    {
        self.0.request(self, path)
    }
}

struct ResourcesInner {
    pub file_system: fs::FileSystem,
    types: RwLock<HashMap<std::any::TypeId, Box<dyn Any>>>,
}

unsafe impl Send for ResourcesInner {}
unsafe impl Sync for ResourcesInner {}

impl ResourcesInner {
    fn new(data_dir: impl AsRef<Path>) -> Result<Self, ()> {
        let mut file_system = fs::FileSystem::default();
        file_system.push_layer(fs::OsFileSystemLayer::new(data_dir.as_ref()));
        file_system.push_layer(fs::GutFileSystemLayer::new(data_dir.as_ref()));

        Ok(Self {
            file_system,
            types: RwLock::new(HashMap::default()),
        })
    }

    fn request<R>(&self, resources: &Resources, path: impl AsRef<Path>) -> Result<Resource<R>, ()>
    where
        R: ResourceType + 'static,
    {
        let type_id = std::any::TypeId::of::<R>();

        {
            // Make sure the type exists in the map.
            let mut types = self.types.write();
            types
                .entry(type_id)
                .or_insert(Box::new(ResourceCache::<R>::default()));
        }

        let path_buf = path.as_ref().to_path_buf();

        {
            let types = self.types.read();
            if let Some(resource) = types[&type_id]
                .downcast_ref::<ResourceCache<R>>()
                .unwrap()
                .resources
                .get(&path_buf)
            {
                return Ok(Resource::clone(resource));
            }
        }

        let data = self.file_system.load(&path_buf).map_err(|_| ())?;
        let context = ResourceLoadContext {
            resources: resources.clone(),
            path: path_buf.clone(),
        };
        let resource = Resource {
            resources: resources.clone(),
            data: Arc::new(R::from_data(data, &context)?),
        };

        {
            let mut types = self.types.write();
            // Cache the new resource.
            types
                .get_mut(&type_id)
                .unwrap()
                .downcast_mut::<ResourceCache<R>>()
                .unwrap()
                .resources
                .insert(path_buf, resource.clone());
        }

        Ok(resource)
    }
}

struct ResourceCache<R: ResourceType> {
    resources: HashMap<PathBuf, Resource<R>>,
}

impl<R: ResourceType> Default for ResourceCache<R> {
    fn default() -> Self {
        Self {
            resources: HashMap::default(),
        }
    }
}
