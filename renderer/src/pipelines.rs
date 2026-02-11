use crate::ShaderId;

/// Handle to a stored color target descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ColorTargetId(pub generational_arena::Index);

/// Handle to a stored render pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RenderPipelineId(pub generational_arena::Index);

/// Minimal color target descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ColorTargetDescriptor {
    pub format: TextureFormat,
}

/// Minimal render pipeline descriptor with ID-based references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderPipelineDescriptor {
    pub vertex_module: ShaderId,
    pub vertex_entry_point: String,
    pub fragment_module: ShaderId,
    pub fragment_entry_point: String,
    pub output_targets: Vec<ColorTargetId>,
}

/// Renderer-owned texture format for pipeline output targets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextureFormat {
    Rgba8UnormSrgb,
    Bgra8UnormSrgb,
    Rgba16Float,
    R16Float,
}

impl TextureFormat {
    /// Converts this format into `wgpu::TextureFormat`.
    pub fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
            Self::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
            Self::R16Float => wgpu::TextureFormat::R16Float,
        }
    }
}

pub(crate) struct ColorTargetEntry {
    pub descriptor: ColorTargetDescriptor,
}

pub(crate) struct RenderPipelineEntry {
    pub descriptor: RenderPipelineDescriptor,
    pub vertex_layout: crate::VertexBufferLayout,
}

/// Converts a renderer color target descriptor into `wgpu::ColorTargetState`.
pub fn to_wgpu_color_target_state(descriptor: ColorTargetDescriptor) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format: descriptor.format.to_wgpu(),
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }
}
