use std::marker::PhantomData;

pub struct Handle<T>(usize, PhantomData<T>);

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

pub struct Storage<T>(slab::Slab<T>);

impl<T> Default for Storage<T> {
    #[inline]
    fn default() -> Self {
        Self(slab::Slab::default())
    }
}

impl<T> Storage<T> {
    #[inline]
    pub fn insert(&mut self, item: T) -> Handle<T> {
        Handle(self.0.insert(item), PhantomData)
    }

    #[inline]
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.0.get(handle.0)
    }

    #[inline]
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.0.get_mut(handle.0)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.0.iter().map(|(h, t)| (Handle(h, PhantomData), t))
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle<T>, &mut T)> {
        self.0.iter_mut().map(|(h, t)| (Handle(h, PhantomData), t))
    }
}
