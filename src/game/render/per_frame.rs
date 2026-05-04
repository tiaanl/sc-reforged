const FRAME_COUNT: usize = 3;

pub struct PerFrame<T> {
    data: [T; FRAME_COUNT],
    index: usize,
}

impl<T> PerFrame<T> {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(usize) -> T,
    {
        Self {
            data: std::array::from_fn(f),
            index: 0,
        }
    }

    pub fn current(&self) -> &T {
        &self.data[self.index]
    }

    pub fn current_mut(&mut self) -> &mut T {
        &mut self.data[self.index]
    }

    pub fn advance(&mut self) -> &mut T {
        self.index = (self.index + 1) % FRAME_COUNT;
        self.current_mut()
    }
}
