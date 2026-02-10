pub use wgpu::BufferUsages;

pub struct BufferId(pub generational_arena::Index);

pub struct BufferDescriptor {
    pub label: String,
    pub size: u64,
    pub usages: BufferUsages,
}
