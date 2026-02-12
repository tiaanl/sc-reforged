use strum::EnumString;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, EnumString)]
pub enum State {
    #[default]
    None,
    #[strum(serialize = "MSEQ_STATE_STAND")]
    Stand,
    #[strum(serialize = "MSEQ_STATE_CROUCH")]
    Crouch,
    #[strum(serialize = "MSEQ_STATE_PRONE")]
    Prone,
    #[strum(serialize = "MSEQ_STATE_ON_BACK")]
    OnBack,
    #[strum(serialize = "MSEQ_STATE_SIT")]
    Sit,
    #[strum(serialize = "MSEQ_STATE_SCUBA")]
    Scuba,
}

impl State {
    /// Convert a raw motion-state id into a semantic [State].
    ///
    /// Unknown ids are treated as [State::None].
    pub const fn from_motion_state_id(id: u32) -> Self {
        match id {
            1 => Self::Stand,
            2 => Self::Crouch,
            3 => Self::Prone,
            4 => Self::OnBack,
            5 => Self::Sit,
            6 => Self::Scuba,
            _ => Self::None,
        }
    }
}
