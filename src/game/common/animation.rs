use ahash::HashMap;

use crate::Transform;

#[derive(Debug, Default)]
pub struct AnimationSet {
    pub set: HashMap<usize, Transform>,
}
