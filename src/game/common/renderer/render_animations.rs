use glam::{Mat4, Quat, Vec3};

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
    pos: wgpu::Texture,
    rot: wgpu::Texture,
}

#[derive(Default)]
pub struct RenderAnimations {
    animations: Storage<RenderAnimation>,
}

impl RenderAnimations {
    pub fn add(
        &mut self,
        animation_handle: Handle<Animation>,
        nodes: &[Node],
    ) -> Handle<RenderAnimation> {
        let animation = animations()
            .get(animation_handle)
            .expect("Adding missing animation!");

        let baked_animation = Self::bake_animation_globals(animation, nodes, 30.0, 10, true);

        let create_texture = |data: &[u8]| {
            let size = wgpu::Extent3d {
                width: baked_animation.frames,
                height: baked_animation.bones,
                depth_or_array_layers: 1,
            };

            let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
                label: Some("animation"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });

            let layout = wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(size.width * 8),
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
        };

        let pos = create_texture(bytemuck::cast_slice(&baked_animation.pos_rgba16f));
        let rot = create_texture(bytemuck::cast_slice(&baked_animation.rot_rgba16f));

        self.animations.insert(RenderAnimation { pos, rot })
    }

    pub fn bake_animation_globals(
        anim: &Animation, // your pos/rot track split or Transform track
        nodes: &[Node],   // for hierarchy + bind locals
        fps: f32,
        frames: u32, // total frames to bake (e.g., last_key_frame + 1)
        looping: bool,
    ) -> BakedAnimation {
        let bones = nodes.len() as u32;
        let mut pos = vec![0u16; (frames * bones * 4) as usize];
        let mut rot = vec![0u16; (frames * bones * 4) as usize];

        // Precompute parent indices
        let parent_idx: Vec<i32> = nodes.iter().map(|n| n.parent as i32).collect();

        // Bake each frame
        for f in 0..frames {
            let t = f as f32 / fps;

            // 1) Sample locals per bone (absolute locals; if you stored deltas, compose with bind here)
            let locals: Vec<Transform> = anim.sample_pose(t, nodes, looping);

            // 2) Build GLOBALS by walking the hierarchy
            let mut globals = vec![Mat4::IDENTITY; nodes.len()];
            for (i, node) in nodes.iter().enumerate() {
                let m_local =
                    Mat4::from_rotation_translation(locals[i].rotation, locals[i].translation);
                globals[i] = if parent_idx[i] < 0 {
                    m_local
                } else {
                    globals[parent_idx[i] as usize] * m_local
                };
            }

            // 3) Write globals to textures (xyz pos, xyzw rot)
            for (i, g) in globals.iter().enumerate() {
                let (translation, rotation) = {
                    // Extract TR from matrix (no scale)
                    let t3 = g.transform_point3(Vec3::ZERO);
                    // If you prefer to store local TR instead, take from `locals[i]`
                    let r = Quat::from_mat4(g).normalize(); // glam can derive quat from mat
                    (t3, r)
                };

                // ensure quaternion continuity across frames per bone (optional if you’ll hemisphere-fix on GPU)
                // you can track last per-bone quat and flip sign if dot < 0 to make GPU nlerp safe

                let base = ((f * bones + (i as u32)) * 4) as usize;

                // pos
                pos[base + 0] = f32_to_f16_bits(translation.x);
                pos[base + 1] = f32_to_f16_bits(translation.y);
                pos[base + 2] = f32_to_f16_bits(translation.z);
                pos[base + 3] = 0;

                // rot
                rot[base + 0] = f32_to_f16_bits(rotation.x);
                rot[base + 1] = f32_to_f16_bits(rotation.y);
                rot[base + 2] = f32_to_f16_bits(rotation.z);
                rot[base + 3] = f32_to_f16_bits(rotation.w);
            }
        }

        fn f32_to_f16_bits(x: f32) -> u16 {
            // use `half` crate normally: half::f16::from_f32(x).to_bits()
            half::f16::from_f32(x).to_bits()
        }

        BakedAnimation {
            frames,
            bones,
            pos_rgba16f: pos,
            rot_rgba16f: rot,
        }
    }
}

#[derive(Debug)]
pub struct BakedAnimation {
    pub frames: u32,
    pub bones: u32,
    pub pos_rgba16f: Vec<u16>, // len = frames * bones * 4
    pub rot_rgba16f: Vec<u16>, // len = frames * bones * 4
}
