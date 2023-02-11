use strum_macros::AsRefStr;

#[derive(Copy, Clone, PartialEq, Eq, Hash, AsRefStr)]
pub enum ResourceColumn {
    Namespace,
    Name,
    TYpe,
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
