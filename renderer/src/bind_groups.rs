/// Description of a single bind group layout entry using renderer-owned types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindGroupLayoutEntry {
    pub binding: u32,
    pub visibility: ShaderStages,
    pub ty: BindingType,
}

/// Shader stage visibility for a bind group entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShaderStages {
    Vertex,
    Fragment,
    VertexFragment,
}

/// Binding type for a bind group entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingType {
    UniformBuffer,
}

pub trait AsBindGroup {
    /// Returns the bind group layout entries declared by this type.
    fn layout_entries() -> &'static [BindGroupLayoutEntry];
}
