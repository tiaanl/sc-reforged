use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::{
    engine::{bind_group::BindGroup, prelude::renderer},
    game::{
        camera::{Camera, GpuCamera},
        math::ViewProjection,
    },
};

pub struct Cascade {
    /// Index in the list of cascades.
    index: u32,
    /// An orthogonal view projection covering part of the camera frustum.
    pub view_projection: ViewProjection,
    /// GPU camera used to render the shadow map.
    pub gpu_camera: GpuCamera,
}

impl Cascade {
    fn new(device: &wgpu::Device, index: u32) -> Self {
        Self {
            index,
            view_projection: ViewProjection::default(),
            gpu_camera: GpuCamera::new(device),
        }
    }
}

pub struct ShadowCascades {
    /// Pixel resolution of the shadow maps.
    pub resolution: u32,
    /// A view projection over the entire camera frustum from the light's direction.
    pub full_view_projection: ViewProjection,
    /// List of cascades for rendering.
    pub cascades: Vec<Cascade>,
    /// Contains a texture array with `count` layers for each shadow depth buffer.
    shadow_buffers: wgpu::Texture,
    /// GPU buffer holding the cascades data.
    cascades_buffer: wgpu::Buffer,
    /// Holds all the data needed for rendering shadows.
    pub cascades_bind_group: BindGroup,
    /// Holds the shadow maps and sampler.
    pub shadow_maps_bind_group: BindGroup,
}

impl ShadowCascades {
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub const MAX_CASCADES: usize = 4;

    pub fn depth_stencil_state() -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: Self::FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }
    }

    pub fn new(device: &wgpu::Device, count: u32, resolution: u32) -> Self {
        debug_assert!(count > 0 && count as usize <= Self::MAX_CASCADES);
        debug_assert!(resolution > 0);

        tracing::info!("Creating {count} shadow cascades at {resolution}x{resolution}.");

        let cascades = (0..count).map(|i| Cascade::new(device, i)).collect();

        let size = wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: count,
        };

        let shadow_maps = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_maps_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let shadow_maps_view = shadow_maps.create_view(&wgpu::TextureViewDescriptor {
            label: Some("shadow_maps_texture_view"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            base_array_layer: 0,
            array_layer_count: Some(count),
            ..Default::default()
        });

        let shadow_maps_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_cascades_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear, // PCF
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let shadow_maps_bind_group = {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow_maps_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow_maps_bind_group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&shadow_maps_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&shadow_maps_sampler),
                    },
                ],
            });

            BindGroup { layout, bind_group }
        };

        let gpu_cascades = GpuCascades::default();
        let cascades_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_cascades_buffer"),
            contents: bytemuck::bytes_of(&gpu_cascades),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let cascades_bind_group = {
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow_cascades_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow_cascades_bind_group"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cascades_buffer.as_entire_binding(),
                }],
            });

            BindGroup { layout, bind_group }
        };

        Self {
            resolution,
            full_view_projection: ViewProjection::default(),
            cascades,
            cascades_buffer,
            shadow_buffers: shadow_maps,
            cascades_bind_group,
            shadow_maps_bind_group,
        }
    }

    pub fn clear_buffers(&self, encoder: &mut wgpu::CommandEncoder) {
        for cascade in self.cascades.iter() {
            let label = format!("clear_shadow_cascade_{}", cascade.index);
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&label),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.cascade_view(cascade.index),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
    }

    pub fn update_from_camera(&mut self, camera: &Camera, light_direction: Vec3, lambda: f32) {
        let guard_xy = 50.0;
        let guard_near = 50.0;
        let guard_far = 50.0;

        let slice_planes = camera.view_slice_planes(self.cascades.len() as u32, lambda);

        let near = slice_planes.first().unwrap();
        let far = slice_planes.last().unwrap();
        let corners: &[[Vec3; 4]] = &[*near, *far];

        self.full_view_projection = self.fit_directional_light(
            light_direction,
            corners,
            self.resolution,
            guard_xy,
            guard_near,
            guard_far,
        );

        let mut gpu_cascades = GpuCascades {
            count: self.cascades.len() as u32,
            ..Default::default()
        };

        for (index, corners) in slice_planes.windows(2).enumerate() {
            let view_projection = self.fit_directional_light(
                light_direction,
                corners,
                self.resolution,
                guard_xy,
                guard_near,
                guard_far,
            );

            let cascade = &mut self.cascades[index];
            debug_assert!(cascade.index == index as u32);

            cascade.view_projection = view_projection.clone();
            cascade.gpu_camera.upload(&view_projection, Vec3::ZERO);

            gpu_cascades.light_view_projection[index] = view_projection.mat.to_cols_array();
        }

        renderer()
            .queue
            .write_buffer(&self.cascades_buffer, 0, bytemuck::bytes_of(&gpu_cascades));
    }

    pub fn fit_directional_light(
        &self,
        sun_dir: Vec3, // direction from sun toward world
        corners: &[[Vec3; 4]],
        shadow_res: u32, // e.g. 2048
        guard_xy: f32,   // extra margin around frustum in world units
        guard_z_near: f32,
        guard_z_far: f32,
    ) -> ViewProjection {
        debug_assert!(corners.len() == 2);

        // Build light view
        let forward = sun_dir.normalize();
        let mut up = Vec3::Z;
        if forward.abs_diff_eq(up, 1e-4) {
            up = Vec3::Y;
        }

        // Place eye at frustum center - some distance back along light dir
        let center = corners[0]
            .iter()
            .chain(&corners[1])
            .copied()
            .reduce(|a, b| a + b)
            .unwrap()
            / 8.0;
        let eye = center - forward * 10_000.0; // far enough to see everything
        let view = Mat4::look_at_lh(eye, center, up);

        // Transform corners into light space
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for &p in corners[0].iter().chain(&corners[1]) {
            let point = view.transform_point3(p);
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
            min_z = min_z.min(point.z);
            max_z = max_z.max(point.z);
        }

        // Add guard bands
        min_x -= guard_xy;
        max_x += guard_xy;
        min_y -= guard_xy;
        max_y += guard_xy;
        min_z -= guard_z_near;
        max_z += guard_z_far;

        {
            // Texel snap.
            let w = max_x - min_x;
            let h = max_y - min_y;
            let step_x = w / shadow_res as f32;
            let step_y = h / shadow_res as f32;

            let cx = 0.5 * (min_x + max_x);
            let cy = 0.5 * (min_y + max_y);
            let cx_snapped = (cx / step_x).floor() * step_x;
            let cy_snapped = (cy / step_y).floor() * step_y;

            let half_w = 0.5 * w;
            let half_h = 0.5 * h;
            min_x = cx_snapped - half_w;
            max_x = cx_snapped + half_w;
            min_y = cy_snapped - half_h;
            max_y = cy_snapped + half_h;
        }

        // Ortho projection (LH, depth 0..1 for wgpu)
        let projection = Mat4::orthographic_lh(min_x, max_x, min_y, max_y, min_z, max_z);

        ViewProjection::from_projection_view(projection, view)
    }

    pub fn cascade_view(&self, index: u32) -> wgpu::TextureView {
        let label = format!("shadow_cascade_view_{index}");
        self.shadow_buffers
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some(&label),
                format: Some(Self::FORMAT),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: index,
                array_layer_count: Some(1),
                usage: Some(wgpu::TextureUsages::RENDER_ATTACHMENT),
                ..Default::default()
            })
    }
}

#[derive(Clone, Copy, Default, bytemuck::NoUninit)]
#[repr(C)]
struct GpuCascades {
    light_view_projection: [[f32; 16]; ShadowCascades::MAX_CASCADES],
    count: u32,
    _pad: [u32; 3],
}
