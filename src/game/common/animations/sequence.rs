use crate::{
    engine::storage::Handle,
    game::{animations::Animation, config},
};

pub struct Clip {
    pub animation: Handle<Animation>,
    pub immediate: bool,
    pub repeat: config::Repeat,
    pub callbacks: Vec<config::Callback>,
}

#[derive(Default)]
pub struct Sequence {
    // The `name` here is just so we can get a name from a handle.  Don't want to reverse lookup
    // from the hash map.
    pub name: String,
    pub clips: Vec<Clip>,
}
