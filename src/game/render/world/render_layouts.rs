use ahash::HashMap;

use crate::engine::renderer::Gpu;

pub trait RenderLayout {
    fn label() -> &'static str;
    fn entries() -> &'static [wgpu::BindGroupLayoutEntry];
}

pub struct RenderLayouts {
    gpu: Gpu,
    layouts: HashMap<std::any::TypeId, wgpu::BindGroupLayout>,
}

impl RenderLayouts {
    pub fn new(gpu: Gpu) -> Self {
        Self {
            gpu,
            layouts: HashMap::default(),
        }
    }

    pub fn get<L: RenderLayout + 'static>(&mut self) -> &wgpu::BindGroupLayout {
        let id = std::any::TypeId::of::<L>();

        self.layouts.entry(id).or_insert_with(|| {
            self.gpu
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(L::label()),
                    entries: L::entries(),
                })
        })
    }
}
