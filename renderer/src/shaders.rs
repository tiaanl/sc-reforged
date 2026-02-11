/// Handle to a shader tracked by the renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ShaderId(pub generational_arena::Index);
