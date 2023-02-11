use strum_macros::AsRefStr;
use strum_macros::EnumIter;

#[derive(Copy, Clone, PartialEq, Eq, Hash, AsRefStr, EnumIter)]
pub enum ResourceColumn {
    Namespace,
    Name,
    Type,
    Ports,
    ClusterIp,
    ExternalIp,
    Ready,
    Restarts,
    Status,
    Ip,
    Node,
    Age,
}
