use glam::Mat4;

use crate::{engine::transform::Transform, game::scenes::world::sim_world::sequences::Pose};

#[derive(Clone, Debug)]
pub struct Bone {
    pub parent: u32,
    pub transform: Transform,
    pub id: u32,
    pub _name: String,
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    pub fn local_transform(&self, bone_index: u32) -> Mat4 {
        let node = &self.bones[bone_index as usize];
        if node.parent == u32::MAX {
            node.transform.to_mat4()
        } else {
            self.local_transform(node.parent) * node.transform.to_mat4()
        }
    }

    pub fn to_pose(&self) -> Pose {
        Pose {
            bones: (0..self.bones.len() as u32)
                .map(|i| self.local_transform(i))
                .collect(),
        }
    }
}
