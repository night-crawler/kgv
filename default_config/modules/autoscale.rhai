fn scale_target(resource) {
    let target = resource?.spec?.scaleTargetRef;
    if target == () {
        return ""
    }

    let parts = [
        target?.apiVersion,
        target?.kind,
        target?.name,
    ];

    parts.filter(|part| part != ()).join("/")
}

fn replicas(resource) {
    let min = resource?.spec?.minReplicas ?? "-";
    let max = resource?.spec?.maxReplicas ?? "-";

    `${min}/${max}`
}


fn pa_scale(resource) {
    let actual = resource?.status?.actualScale ?? "-";
    let desired = resource?.status?.desiredScale ?? "-";

    `${actual}/${desired}`
}