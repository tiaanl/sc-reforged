use bevy_ecs::component::Component;

#[derive(Component)]
pub struct Handle<T>(generational_arena::Index, std::marker::PhantomData<T>);

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

pub struct Storage<T>(generational_arena::Arena<T>);

impl<T> Storage<T> {
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(generational_arena::Arena::with_capacity(capacity))
    }

    #[inline]
    pub fn get(&self, id: Handle<T>) -> Option<&T> {
        self.0.get(id.0)
    }

    #[inline]
    pub fn get_mut(&mut self, id: Handle<T>) -> Option<&mut T> {
        self.0.get_mut(id.0)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.0
            .iter()
            .map(|(index, value)| (Handle(index, std::marker::PhantomData), value))
    }

    #[inline]
    pub fn _iter_mut(&mut self) -> impl Iterator<Item = (Handle<T>, &mut T)> {
        self.0
            .iter_mut()
            .map(|(index, value)| (Handle(index, std::marker::PhantomData), value))
    }

    #[inline]
    pub fn insert(&mut self, value: T) -> Handle<T> {
        Handle(self.0.insert(value), std::marker::PhantomData)
    }
}

impl<T> Default for Storage<T> {
    fn default() -> Self {
        Self(generational_arena::Arena::default())
    }
}
