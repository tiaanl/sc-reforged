use std::marker::PhantomData;

use bevy_ecs::system::Resource;

pub struct Handle<T>(usize, PhantomData<T>);

#[derive(Default, Resource)]
pub struct Storage<T> {
    items: slab::Slab<T>,
}

impl<T> Storage<T> {
    fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.items.get(handle.0)
    }

    fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.items.get_mut(handle.0)
    }

    fn insert(&mut self, item: T) -> Handle<T> {
        Handle(self.items.insert(item), PhantomData)
    }
}
