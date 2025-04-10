use std::marker::PhantomData;

use bevy_ecs::system::Resource;

pub struct Handle<T>(usize, PhantomData<T>);

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

impl<T> Copy for Handle<T> {}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0).finish()
    }
}

#[derive(Resource)]
pub struct Storage<T> {
    items: slab::Slab<T>,
}

impl<T> Default for Storage<T> {
    fn default() -> Self {
        Self {
            items: Default::default(),
        }
    }
}

impl<T> Storage<T> {
    pub fn insert(&mut self, item: T) -> Handle<T> {
        Handle(self.items.insert(item), PhantomData)
    }

    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.items.get(handle.0)
    }

    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.items.get_mut(handle.0)
    }
}
