use glam::{Mat4, Vec3, Vec4};
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
    pub const MAX_CASCADES: usize = 4;
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

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

    pub fn new(device: &wgpu::Device, resolution: u32) -> Self {
        debug_assert!(resolution > 0);

        tracing::info!(
            "Creating {} shadow cascades at {resolution}x{resolution}.",
            Self::MAX_CASCADES
        );

        let cascades = (0..Self::MAX_CASCADES)
            .map(|i| Cascade::new(device, i as u32))
            .collect();

        let size = wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: Self::MAX_CASCADES as u32,
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
            array_layer_count: Some(Self::MAX_CASCADES as u32),
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

        let gpu_cascades = GpuCascades::default();
        let cascades_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_cascades_buffer"),
            contents: bytemuck::bytes_of(&gpu_cascades),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
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
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cascades_buffer.as_entire_binding(),
                    },
                ],
            });

            BindGroup { layout, bind_group }
        };

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
            camera,
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
                camera,
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

    #[allow(clippy::too_many_arguments)]
    pub fn fit_directional_light(
        &self,
        sun_dir: Vec3, // direction from sun toward world
        camera: &Camera,
        corners: &[[Vec3; 4]], // [near4, far4] of the slice in WORLD
        shadow_res: u32,       // e.g. 2048
        guard_xy: f32,         // extra margin around frustum in world units
        guard_z_near: f32,
        guard_z_far: f32,
    ) -> ViewProjection {
        debug_assert!(corners.len() == 2);

        // --- Camera-aligned light basis -----------------------------------------
        // Camera world axes (match on-screen orientation)
        let inv_view = camera.calculate_view().inverse();
        let camera_right = inv_view.col(0).truncate(); // world-space right
        let camera_up = inv_view.col(1).truncate(); // world-space up

        // Light forward along the sun (LH: +Z forward)
        let light_forward = sun_dir.normalize();

        // Project camera axes into plane orthogonal to light_forward
        let mut light_right = camera_right - light_forward * camera_right.dot(light_forward);
        if light_right.length_squared() < 1e-8 {
            // If degenerate, fall back to camera_up
            light_right = camera_up - light_forward * camera_up.dot(light_forward);
        }
        if light_right.length_squared() < 1e-8 {
            // Final fallback: pick any vector not parallel to forward
            let aux = if light_forward.abs_diff_eq(Vec3::Z, 1e-4) {
                Vec3::X
            } else {
                Vec3::Z
            };
            light_right = aux - light_forward * aux.dot(light_forward);
        }
        light_right = light_right.normalize();

        // LH basis: up = forward × right
        let mut light_up = light_forward.cross(light_right);
        if light_up.length_squared() < 1e-8 {
            // Extremely rare; enforce orthonormality
            light_up = light_forward.cross(Vec3::X).normalize();
            light_right = light_up.cross(light_forward).normalize();
        } else {
            light_up = light_up.normalize();
        }

        // Slice center as the "eye" for tight/stable depth
        let center = corners[0]
            .iter()
            .chain(&corners[1])
            .copied()
            .reduce(|a, b| a + b)
            .unwrap()
            / 8.0;

        // Build view from basis and eye (LH; columns = right, up, forward, translation)
        let view = Mat4::from_cols(
            Vec4::new(light_right.x, light_up.x, light_forward.x, 0.0),
            Vec4::new(light_right.y, light_up.y, light_forward.y, 0.0),
            Vec4::new(light_right.z, light_up.z, light_forward.z, 0.0),
            Vec4::new(
                -light_right.dot(center),
                -light_up.dot(center),
                -light_forward.dot(center),
                1.0,
            ),
        );

        // --- Fit AABB of the slice in this camera-aligned light space ------------
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for &p in corners[0].iter().chain(&corners[1]) {
            let q = view.transform_point3(p);
            min_x = min_x.min(q.x);
            max_x = max_x.max(q.x);
            min_y = min_y.min(q.y);
            max_y = max_y.max(q.y);
            min_z = min_z.min(q.z);
            max_z = max_z.max(q.z);
        }

        // --- Guard bands ----------------------------------------------------------
        min_x -= guard_xy;
        max_x += guard_xy;
        min_y -= guard_xy;
        max_y += guard_xy;
        min_z -= guard_z_near; // toward the light origin
        max_z += guard_z_far; // away from the origin

        // --- Texel snapping (stabilize when camera pans) -------------------------
        {
            let w = (max_x - min_x).max(1e-6);
            let h = (max_y - min_y).max(1e-6);
            let step_x = w / shadow_res as f32;
            let step_y = h / shadow_res as f32;

            // Snap the center to the texel grid; preserve size
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

        // --- Ortho projection (LH, z ∈ [0,1] for wgpu) ---------------------------
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
