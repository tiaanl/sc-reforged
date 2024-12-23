use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc, RwLock},
};

pub trait Asset {}

pub struct Handle<A: Asset>(u64, std::marker::PhantomData<A>);

impl<A: Asset> Handle<A> {
    pub fn from_raw(id: u64) -> Self {
        Self(id, std::marker::PhantomData)
    }

    pub fn as_raw(&self) -> u64 {
        self.0
    }
}

impl<A: Asset> Copy for Handle<A> {}

impl<A: Asset> std::hash::Hash for Handle<A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        // Just compare the ID's
        self.0 == other.0
    }
}

impl<A: Asset> Eq for Handle<A> {}

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: Asset> std::fmt::Debug for Handle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle({})", self.0)
    }
}

#[derive(Clone, Default)]
pub struct AssetStore {
    next_id: Arc<AtomicU64>,
    storages: Arc<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

impl AssetStore {
    fn generate_handle<A>(&self) -> Handle<A>
    where
        A: Asset + Send + Sync + 'static,
    {
        use std::sync::atomic::Ordering;

        Handle(
            self.next_id.fetch_add(1, Ordering::Relaxed),
            std::marker::PhantomData,
        )
    }

    pub fn add<A>(&self, asset: A) -> Handle<A>
    where
        A: Asset + Send + Sync + 'static,
    {
        let handle = self.generate_handle::<A>();

        // Access or create the storage for the asset type
        let mut storages = self.storages.write().unwrap();
        let type_id = TypeId::of::<A>();
        let storage = storages
            .entry(type_id)
            .or_insert_with(|| Box::new(RwLock::new(HashMap::<u64, Arc<A>>::default())))
            .downcast_mut::<RwLock<HashMap<u64, Arc<A>>>>()
            .expect("Type mismatch in storage");

        // Add the asset to the storage
        storage.write().unwrap().insert(handle.0, Arc::new(asset));
        handle
    }

    // Retrieve an asset of type A
    pub fn get<A>(&self, handle: Handle<A>) -> Option<Arc<A>>
    where
        A: Asset + Send + Sync + 'static,
    {
        let storages = self.storages.read().unwrap();
        let type_id = TypeId::of::<A>();

        // Access the storage for the requested type
        storages.get(&type_id).and_then(|storage| {
            storage
                .downcast_ref::<RwLock<HashMap<u64, Arc<A>>>>()
                .expect("Type mismatch in storage")
                .read()
                .unwrap()
                .get(&handle.0)
                .cloned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Number(i32);
    impl Asset for Number {}

    #[test]
    fn basic() {
        let store = AssetStore::default();
        let h = store.add(Number(10));
        let maybe = store.get(h);
        assert!(maybe.is_some());
        let value = maybe.unwrap();
        assert_eq!(value.0, 10);
    }
}
