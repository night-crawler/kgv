fn system(resource) {
    let parts = [
        resource?.status?.nodeInfo?.operatingSystem,
        resource?.status?.nodeInfo?.architecture,
        resource?.status?.nodeInfo?.kernelVersion,
    ];
    let parts = parts.filter(|part| part != ());
    parts.join(" ")
}

fn cpu(resource) {
    let parts = [
        resource?.status?.capacity?.cpu ?? "0",
        resource?.status?.allocatable?.cpu ?? "0",
    ];
    parts.join("/")
}

fn pods(resource) {
    let cap = resource?.status?.capacity?.pods ?? 0;
    let alloc = resource?.status?.allocatable?.pods ?? 0;

    if cap == alloc {
        `${cap}`
    } else {
        `${cap}/${alloc}`
    }
}

fn total_image_gb(resource) {
    let total = 0;
    for img in (resource?.status?.images ?? []) {
        total += img.sizeBytes ?? 0;
    }

    let gb = total / 1024 / 1024 / 1024;
    gb
}