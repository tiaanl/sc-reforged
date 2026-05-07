use ahash::HashMap;

use crate::game::globals;

pub trait RenderLayout {
    fn label() -> &'static str;
    fn entries() -> &'static [wgpu::BindGroupLayoutEntry];
}

#[derive(Default)]
pub struct RenderLayouts {
    layouts: HashMap<std::any::TypeId, wgpu::BindGroupLayout>,
}

impl RenderLayouts {
    pub fn get<L: RenderLayout + 'static>(&mut self) -> &wgpu::BindGroupLayout {
        let id = std::any::TypeId::of::<L>();

        self.layouts.entry(id).or_insert_with(|| {
            globals::gpu()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some(L::label()),
                    entries: L::entries(),
                })
        })
    }
}
