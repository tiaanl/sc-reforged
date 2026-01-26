pub struct PerFrame<T, const N: usize> {
    data: [T; N],
    index: usize,
}

impl<T, const N: usize> PerFrame<T, N> {
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
        self.index = (self.index + 1) % N;
        self.current_mut()
    }
}
