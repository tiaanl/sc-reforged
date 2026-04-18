use glam::IVec2;

#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub position: IVec2,
    pub size: IVec2,
}

impl Rect {
    pub fn new(position: IVec2, size: IVec2) -> Self {
        Self { position, size }
    }

    #[must_use]
    pub fn from_position(position: IVec2) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn from_size(size: IVec2) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_position(self, position: IVec2) -> Self {
        Self {
            position,
            size: self.size,
        }
    }

    #[must_use]
    pub fn with_size(self, size: IVec2) -> Self {
        Self {
            position: self.position,
            size,
        }
    }

    #[must_use]
    pub fn offset(self, offset: IVec2) -> Self {
        Self {
            position: self.position + offset,
            size: self.size,
        }
    }

    #[must_use]
    pub fn grow(self, size: IVec2) -> Self {
        Self {
            position: self.position,
            size: self.size + size,
        }
    }

    #[must_use]
    pub fn intersection(self, other: Self) -> Self {
        let min = self.position.max(other.position);
        let max = (self.position + self.size).min(other.position + other.size);
        let size = (max - min).max(IVec2::ZERO);

        Self {
            position: min,
            size,
        }
    }
}
