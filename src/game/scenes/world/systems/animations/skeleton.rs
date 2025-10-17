use super::BoneIndex;

#[derive(Clone)]
pub struct Bone {
    pub name: String,
    pub parent: BoneIndex,
}

pub struct Skeleton {
    pub bones: Vec<Bone>,
}

impl Skeleton {
    pub fn from_slice(bones: &[Bone]) -> Self {
        Self {
            bones: bones.to_vec(),
        }
    }

    pub fn from_iter(bones: impl Iterator<Item = Bone>) -> Self {
        Self {
            bones: bones.collect(),
        }
    }
}
