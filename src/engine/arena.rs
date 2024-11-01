#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Handle<T>(usize, std::marker::PhantomData<T>);

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
