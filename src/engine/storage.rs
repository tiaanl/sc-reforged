use std::marker::PhantomData;

pub struct Handle<T>(usize, PhantomData<T>);

impl<T> Handle<T> {
    pub fn _from_raw(raw: usize) -> Self {
        Self(raw, PhantomData)
    }

    pub fn _as_raw(&self) -> usize {
        self.0
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        // Just compare the ID's
        self.0 == other.0
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0).finish()
    }
}

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
