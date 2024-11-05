#[derive(Copy, Eq, Hash, PartialEq)]
pub struct Handle<T>(usize, std::marker::PhantomData<T>);

impl<T> Clone for Handle<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0, std::marker::PhantomData)
    }
}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.0).finish()
    }
}

impl<T> Handle<T> {
    pub fn raw(id: usize) -> Self {
        Self(id, std::marker::PhantomData)
    }
}

pub struct Arena<T> {
    pub storage: Vec<T>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self { storage: vec![] }
    }
}

impl<T> Arena<T> {
    pub fn insert(&mut self, value: T) -> Handle<T> {
        let id = self.storage.len();
        self.storage.push(value);
        Handle(id, std::marker::PhantomData::<T>)
    }

    pub fn get(&self, handle: &Handle<T>) -> Option<&T> {
        self.storage.get(handle.0)
    }

    pub fn get_mut(&mut self, handle: &Handle<T>) -> Option<&mut T> {
        self.storage.get_mut(handle.0)
    }
}
