pub use wgpu::BufferUsages;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct BufferId(pub generational_arena::Index);

pub struct BufferDescriptor {
    pub label: String,
    pub size: u64,
    pub usages: BufferUsages,
}
