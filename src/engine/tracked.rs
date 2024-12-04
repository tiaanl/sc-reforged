use std::cell::Cell;

pub struct Tracked<T> {
    value: T,
    changed: Cell<bool>,
}

/// A value that can track whether it has been changed.
impl<T> Tracked<T> {
    /// Create a new value with a changed status.
    pub fn new(value: T) -> Self {
        Self {
            value,
            changed: Cell::new(true),
        }
    }

    pub fn reset(&mut self) {
        self.changed.replace(false);
    }

    /// Returns true if the value was changed.
    pub fn changed(&self) -> bool {
        self.changed.get()
    }

    /// Call the function with a reference to the value if the value was changed and reset the
    /// status to unchanged if it was.
    pub fn if_changed(&self, mut f: impl FnMut(&T)) {
        if self.changed.get() {
            f(self);
            self.changed.replace(false);
        }
    }

    /// Call the function with a mutable reference to the value if the value was changed and reset
    /// the status to unchanged if it was.
    pub fn if_changed_mut(&mut self, mut f: impl FnMut(&mut T)) {
        if self.changed.get() {
            f(self);
            self.changed.replace(false);
        }
    }
}

impl<T> std::ops::Deref for Tracked<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for Tracked<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed.replace(true);
        &mut self.value
    }
}

impl<T: Default> Default for Tracked<T> {
    fn default() -> Self {
        Self {
            value: Default::default(),
            changed: Cell::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let tracked = Tracked::new(5);
        assert_eq!(*tracked, 5);
        assert!(tracked.changed());
    }

    #[test]
    fn test_reset() {
        let mut tracked = Tracked::new(5);
        tracked.reset();
        assert!(!tracked.changed());
    }

    #[test]
    fn test_if_changed() {
        let tracked = Tracked::new(5);
        let mut called = false;
        tracked.if_changed(|value| {
            assert_eq!(*value, 5);
            called = true;
        });
        assert!(called);
        assert!(!tracked.changed());
    }

    #[test]
    fn test_if_changed_mut() {
        let mut tracked = Tracked::new(5);
        let mut called = false;
        tracked.if_changed_mut(|value| {
            assert_eq!(*value, 5);
            *value = 10;
            called = true;
        });
        assert!(called);
        assert!(!tracked.changed());
        assert_eq!(*tracked, 10);
    }

    #[test]
    fn test_deref() {
        let tracked = Tracked::new(5);
        assert_eq!(*tracked, 5);
    }

    #[test]
    fn test_deref_mut() {
        let mut tracked = Tracked::new(5);
        *tracked = 10;
        assert_eq!(*tracked, 10);
        assert!(tracked.changed());
    }
}
