fn status_replicas(resource) {
    let replicas = resource?.spec?.replicas ?? "-";
    let parts = [
        resource?.status?.readyReplicas ?? "-",
        resource?.status?.currentReplicas ?? "-",
        resource?.status?.updatedReplicas ?? "-",
    ];

    let joined = parts.join("/");

    `${replicas} ${joined}`
}
