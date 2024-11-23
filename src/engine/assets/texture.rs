use super::Asset;

pub struct TextureView {
    pub view: wgpu::TextureView,
}

impl From<wgpu::TextureView> for TextureView {
    fn from(value: wgpu::TextureView) -> Self {
        Self { view: value }
    }
}

impl Asset for TextureView {}

pub struct TextureBindGroup(pub wgpu::BindGroup);

impl Asset for TextureBindGroup {}

impl From<wgpu::BindGroup> for TextureBindGroup {
    fn from(value: wgpu::BindGroup) -> Self {
        Self(value)
    }
}
