/// Describes a vertex buffer layout using renderer-owned types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VertexBufferLayout {
    pub array_stride: u64,
    pub step_mode: VertexStepMode,
    pub attributes: &'static [VertexAttribute],
}

impl VertexBufferLayout {
    /// Converts this layout into `wgpu::VertexBufferLayout`.
    ///
    /// `wgpu_attributes` should be created from `self.attributes` and must live
    /// at least as long as the returned `wgpu::VertexBufferLayout`.
    pub fn to_wgpu<'a>(
        &self,
        wgpu_attributes: &'a [wgpu::VertexAttribute],
    ) -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: self.step_mode.to_wgpu(),
            attributes: wgpu_attributes,
        }
    }
}

/// Trait implemented by types that can provide a vertex buffer layout.
pub trait AsVertexLayout {
    /// Returns the vertex buffer layout for this type.
    fn vertex_buffer_layout() -> VertexBufferLayout;
}

/// Describes one vertex attribute using renderer-owned types.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VertexAttribute {
    pub format: VertexFormat,
    pub offset: u64,
    pub shader_location: u32,
}

impl VertexAttribute {
    /// Converts this attribute into `wgpu::VertexAttribute`.
    pub fn to_wgpu(self) -> wgpu::VertexAttribute {
        wgpu::VertexAttribute {
            format: self.format.to_wgpu(),
            offset: self.offset,
            shader_location: self.shader_location,
        }
    }
}

/// Converts renderer vertex attributes into `wgpu` vertex attributes.
pub fn to_wgpu_vertex_attributes(attributes: &[VertexAttribute]) -> Vec<wgpu::VertexAttribute> {
    attributes
        .iter()
        .copied()
        .map(VertexAttribute::to_wgpu)
        .collect()
}

/// Vertex input stepping mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexStepMode {
    Vertex,
    Instance,
}

impl VertexStepMode {
    /// Converts this step mode into `wgpu::VertexStepMode`.
    pub fn to_wgpu(self) -> wgpu::VertexStepMode {
        match self {
            Self::Vertex => wgpu::VertexStepMode::Vertex,
            Self::Instance => wgpu::VertexStepMode::Instance,
        }
    }
}

/// Vertex attribute format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VertexFormat {
    Float32,
    Float32x2,
    Float32x3,
    Float32x4,
    Uint32,
    Uint32x2,
    Uint32x3,
    Uint32x4,
    Sint32,
    Sint32x2,
    Sint32x3,
    Sint32x4,
}

impl VertexFormat {
    /// Converts this format into `wgpu::VertexFormat`.
    pub fn to_wgpu(self) -> wgpu::VertexFormat {
        match self {
            Self::Float32 => wgpu::VertexFormat::Float32,
            Self::Float32x2 => wgpu::VertexFormat::Float32x2,
            Self::Float32x3 => wgpu::VertexFormat::Float32x3,
            Self::Float32x4 => wgpu::VertexFormat::Float32x4,
            Self::Uint32 => wgpu::VertexFormat::Uint32,
            Self::Uint32x2 => wgpu::VertexFormat::Uint32x2,
            Self::Uint32x3 => wgpu::VertexFormat::Uint32x3,
            Self::Uint32x4 => wgpu::VertexFormat::Uint32x4,
            Self::Sint32 => wgpu::VertexFormat::Sint32,
            Self::Sint32x2 => wgpu::VertexFormat::Sint32x2,
            Self::Sint32x3 => wgpu::VertexFormat::Sint32x3,
            Self::Sint32x4 => wgpu::VertexFormat::Sint32x4,
        }
    }
}
