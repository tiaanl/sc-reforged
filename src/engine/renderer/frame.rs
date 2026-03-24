/// A single object passed around during the rendering of a single frame.
pub struct Frame {
    /// The encoder to use for creating render passes.
    pub encoder: wgpu::CommandEncoder,

    /// The window surface.
    pub surface: wgpu::TextureView,

    /// The index of the frame being rendered.
    pub frame_index: u64,

    /// The size of the surface.
    pub size: glam::UVec2,
}
