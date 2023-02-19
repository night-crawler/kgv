use std::fmt::{Display, Formatter};
use strum_macros::AsRefStr;
use strum_macros::EnumIter;

#[derive(Copy, Clone, PartialEq, Eq, Hash, AsRefStr, EnumIter)]
pub enum PodContainerColumn {
    Name,
    Image,
    Ready,
    State,
    Init,
    Restarts,
    Probes,
    Cpu,
    Mem,
    Ports,
    Age,
}

impl Display for PodContainerColumn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PodContainerColumn::Restarts => "🗘",
            _ => self.as_ref(),
        };
        write!(f, "{}", s)
    }
}
