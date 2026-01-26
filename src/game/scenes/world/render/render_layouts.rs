use ahash::HashMap;

use crate::engine::renderer::Renderer;

pub trait RenderLayout {
    fn label() -> &'static str;
    fn entries() -> &'static [wgpu::BindGroupLayoutEntry];
}

pub struct RenderLayouts {
    layouts: HashMap<std::any::TypeId, wgpu::BindGroupLayout>,
}

impl RenderLayouts {
    pub fn new() -> Self {
        Self {
            layouts: HashMap::default(),
        }
    }

    pub fn get<L: RenderLayout + 'static>(
        &mut self,
        renderer: &Renderer,
    ) -> &wgpu::BindGroupLayout {
        let id = std::any::TypeId::of::<L>();

        self.layouts.entry(id).or_insert_with(|| {
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(L::label()),
                    entries: L::entries(),
                })
        })
    }
}
