fn ready(resource) {
    let available = resource?.status?.availableReplicas ?? 0;
    let replicas = resource?.status?.replicas ?? 0;
    `${available}/${replicas}`
}

