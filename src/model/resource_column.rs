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
    Service,
    Available,
    Ip,
    Node,
    Age,
    Data,

    SignerName,
    Requestor,
    RequestedDuration,
    Condition,

    Secret,
    Drivers,

    Kind,
    FirstLocation,

    UpToDate,
    Desired,
    Current,
    Role,
    Subjects,
    ClusterRole,
    SubjectKind,
    
}
