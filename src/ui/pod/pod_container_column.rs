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
