use std::cell::Cell;

pub struct Dirty(Cell<bool>);

impl Dirty {
    pub fn smudged() -> Self {
        Self(Cell::new(true))
    }

    pub fn clean() -> Self {
        Self(Cell::new(false))
    }

    /// Mark the flag as dirty.
    pub fn smudge(&self) {
        self.0.replace(true);
    }

    pub fn if_dirty(&self, mut f: impl FnMut()) -> bool {
        let changed = self.0.get();
        if changed {
            f();
            self.0.replace(false);
        }
        changed
    }
}

impl Default for Dirty {
    fn default() -> Self {
        Self::clean()
    }
}
