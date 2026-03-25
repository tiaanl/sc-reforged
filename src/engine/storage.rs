use std::marker::PhantomData;

use ahash::HashMap;
use bevy_ecs::component::Component;
use generational_arena::{Arena, Index};

#[derive(Component)]
pub struct Handle<T>(Index, PhantomData<fn() -> T>);

impl<T> Clone for Handle<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> std::fmt::Debug for Handle<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Id").field(&self.0).finish()
    }
}

impl<T> std::fmt::Display for Handle<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> Ord for Handle<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> PartialEq for Handle<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Ignore [PhantomData].
        self.0 == other.0
    }
}

impl<T> PartialOrd for Handle<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Storage<T, Stored = T> {
    arena: Arena<Stored>,
    _marker: PhantomData<fn() -> T>,
}

impl<T, Stored> Storage<T, Stored> {
    #[inline]
    pub fn get(&self, id: Handle<T>) -> Option<&Stored> {
        self.arena.get(id.0)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Handle<T>) -> Option<&mut Stored> {
        self.arena.get_mut(id.0)
    }

    #[inline]
    pub fn insert(&mut self, value: Stored) -> Handle<T> {
        Handle(self.arena.insert(value), PhantomData)
    }
}

impl<T, Stored> Default for Storage<T, Stored> {
    fn default() -> Self {
        Self {
            arena: Arena::default(),
            _marker: PhantomData,
        }
    }
}

pub struct StorageMap<K, T, Stored = T> {
    storage: Storage<T, Stored>,
    lookup: HashMap<K, Handle<T>>,
}

impl<K, T, Stored> Default for StorageMap<K, T, Stored> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            lookup: HashMap::default(),
        }
    }
}

impl<K, T, Stored> StorageMap<K, T, Stored>
where
    K: Eq + std::hash::Hash,
{
    #[inline]
    pub fn get(&self, handle: Handle<T>) -> Option<&Stored> {
        self.storage.get(handle)
    }

    #[inline]
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut Stored> {
        self.storage.get_mut(handle)
    }

    pub fn get_handle_by_key(&self, key: &K) -> Option<Handle<T>> {
        self.lookup.get(key).cloned()
    }

    pub fn get_by_key(&self, key: &K) -> Option<&Stored> {
        self.lookup.get(key).and_then(|h| self.storage.get(*h))
    }

    pub fn get_by_key_mut(&mut self, key: &K) -> Option<&mut Stored> {
        self.lookup.get(key).and_then(|h| self.storage.get_mut(*h))
    }

    pub fn insert(&mut self, key: K, value: Stored) -> Handle<T> {
        let handle = self.storage.insert(value);
        self.lookup.insert(key, handle);
        handle
    }
}
