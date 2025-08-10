use glam::{Quat, Vec3, Vec4};

use crate::{
    engine::{
        prelude::{Transform, renderer},
        storage::{Handle, Storage},
    },
    game::{
        animations::{Animation, animations},
        model::Node,
    },
};

pub struct RenderAnimation {
    pub bind_group: wgpu::BindGroup,
}

pub struct RenderAnimations {
    animations: Storage<RenderAnimation>,

    animation_bind_group_layout: wgpu::BindGroupLayout,
}

impl Default for RenderAnimations {
    fn default() -> Self {
        let animation_bind_group_layout =
            renderer()
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("render_animation_bind_group_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });

        Self {
            animations: Storage::default(),
            animation_bind_group_layout,
        }
    }
}

impl RenderAnimations {
    pub fn get(&self, handle: Handle<RenderAnimation>) -> Option<&RenderAnimation> {
        self.animations.get(handle)
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.animation_bind_group_layout
    }

    pub fn create_rest_pose(&mut self, nodes: &[Node]) -> Handle<RenderAnimation> {
        let mut positions: Vec<Vec4> = Vec::with_capacity(nodes.len());
        let mut rotations: Vec<Quat> = Vec::with_capacity(nodes.len());

        for (node_index, node) in nodes.iter().enumerate() {
            if node.parent == u32::MAX {
                positions.push(node.transform.translation.extend(0.0));
                rotations.push(node.transform.rotation.normalize());
            } else {
                let parent = node.parent as usize;
                assert!(
                    parent < node_index,
                    "nodes must be topologically ordered: parent before child"
                );

                let parent_position = positions[parent];
                let parent_rotation = rotations[parent];

                // Rotate local translation by parent's rotation.
                let position =
                    parent_position + (parent_rotation * node.transform.translation).extend(0.0);
                let rotation = (parent_rotation * node.transform.rotation).normalize();

                positions.push(position);
                rotations.push(rotation);
            }
        }

        let positions_view = {
            let positions_texture =
                Self::create_texture(1, nodes.len() as u32, bytemuck::cast_slice(&positions));

            positions_texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let rotations_view = {
            let rotations_texture =
                Self::create_texture(1, nodes.len() as u32, bytemuck::cast_slice(&rotations));

            rotations_texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let bind_group = self.create_bind_group(&positions_view, &rotations_view);

        self.animations.insert(RenderAnimation { bind_group })
    }

    pub fn add(
        &mut self,
        animation_handle: Handle<Animation>,
        nodes: &[Node],
    ) -> Handle<RenderAnimation> {
        let animation = animations()
            .get(animation_handle)
            .expect("Adding missing animation!");

        // If there is no animation data (sometimes used for an animation that just shows the rest
        // pose), then just use the rest pose.
        if animation.last_key_frame() == 0 {
            return self.create_rest_pose(nodes);
        }

        let baked_animation = Self::bake_animation_globals(animation, nodes, 30.0, true);

        let positions_view = {
            let positions_texture = Self::create_texture(
                baked_animation.frames,
                nodes.len() as u32,
                bytemuck::cast_slice(&baked_animation.positions),
            );

            positions_texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let rotations_view = {
            let rotations_texture = Self::create_texture(
                baked_animation.frames,
                nodes.len() as u32,
                bytemuck::cast_slice(&baked_animation.rotations),
            );

            rotations_texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let bind_group = self.create_bind_group(&positions_view, &rotations_view);

        self.animations.insert(RenderAnimation { bind_group })
    }

    pub fn bake_animation_globals(
        anim: &Animation, // your pos/rot track split or Transform track
        nodes: &[Node],   // for hierarchy + bind locals
        fps: f32,
        looping: bool,
    ) -> BakedAnimation {
        #[inline]
        fn compose_tr(a_t: Vec3, a_r: Quat, b_t: Vec3, b_r: Quat) -> (Vec3, Quat) {
            (a_t + a_r * b_t, (a_r * b_r).normalize())
        }

        // Find the biggest key frame.
        let frames = anim.last_key_frame();

        let bones = nodes.len() as u32;
        let texels = (frames * bones) as usize;

        let mut positions = vec![Vec4::ZERO; texels];
        let mut rotations = vec![Quat::IDENTITY; texels];

        // Keep last quaternion per bone to ensure hemisphere continuity
        let mut last_q: Vec<Option<Quat>> = vec![None; bones as usize];

        for f in 0..frames {
            let t_sec = f as f32 / fps;

            // 1) Sample absolute locals for this frame
            let locals: Vec<Transform> = anim.sample_pose(t_sec, nodes, looping);

            // 2) Build globals by walking hierarchy (assumes parent index < child index)
            let mut g_t = vec![Vec3::ZERO; nodes.len()];
            let mut g_q = vec![Quat::IDENTITY; nodes.len()];

            for (i, n) in nodes.iter().enumerate() {
                let lt = locals[i].translation;
                let lr = locals[i].rotation;

                if n.parent == u32::MAX {
                    g_t[i] = lt;
                    g_q[i] = lr.normalize();
                } else {
                    let p = n.parent as usize;
                    let (wt, wr) = compose_tr(g_t[p], g_q[p], lt, lr);
                    g_t[i] = wt;
                    g_q[i] = wr;
                }
            }

            // 3) Optional: hemisphere-fix per bone to keep adjacent frames “close”
            for i in 0..nodes.len() {
                let q = {
                    if let Some(prev) = last_q[i] {
                        let mut q = g_q[i];
                        if prev.dot(q) < 0.0 {
                            q = Quat::from_xyzw(-q.x, -q.y, -q.z, -q.w);
                        }
                        q
                    } else {
                        g_q[i]
                    }
                };
                last_q[i] = Some(q);

                // 4) Store into flattened arrays at [frame, bone]
                let idx = (i as u32 * frames + f) as usize;

                positions[idx].x = g_t[i].x;
                positions[idx].y = g_t[i].y;
                positions[idx].z = g_t[i].z;
                positions[idx].w = 0.0;

                rotations[idx].x = q.x;
                rotations[idx].y = q.y;
                rotations[idx].z = q.z;
                rotations[idx].w = q.w;
            }
        }

        BakedAnimation {
            frames,
            bones,
            positions,
            rotations,
        }
    }

    fn create_texture(width: u32, height: u32, data: &[u8]) -> wgpu::Texture {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };

        let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
            label: Some("animation"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * 16),
            rows_per_image: Some(size.height),
        };

        renderer().queue.write_texture(
            wgpu::TexelCopyTextureInfoBase {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            layout,
            size,
        );

        texture
    }

    fn create_bind_group(
        &self,
        positions_view: &wgpu::TextureView,
        rotations_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        renderer()
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("render_animation_bind_group"),
                layout: &self.animation_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(positions_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(rotations_view),
                    },
                ],
            })
    }
}

#[derive(Debug)]
pub struct BakedAnimation {
    pub frames: u32,
    pub bones: u32,
    pub positions: Vec<Vec4>,
    pub rotations: Vec<Quat>,
}
