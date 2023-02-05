import "commons" as cm;

fn ready(resource) {
    let ready = 0;
    let total = 0;

    let statuses = resource?.status?.containerStatuses ?? []; 

    for status in statuses {
        if status?.ready {
            ready += 1;
        }
        total += 1;
    }
    `${ready}/${total}`
}

fn restarts(resource) {
    let count = 0;
    let container_statuses = resource.status?.containerStatuses ?? [];
    for container_status in container_statuses {
        count += container_status.restartCount ?? 0;
    }

    count
}

fn node_name(resource) {
    resource?.spec?.nodeName ?? ""
}

fn phase(resource) {
    if cm::is_deleted(resource) {
        return "Deleted"
    }
    resource?.status?.phase ?? ""
}

fn ips(resource) {
    let ips = [];
    for ip_map in resource.status?.podIPs ?? [] {
        if "ip" in ip_map {
            ips.push(ip_map.ip);
        }
    }

    ips.reduce(|sum, v| 
        if sum.is_empty() {
            `${v}`
        } else {
            `${sum},${v}`           
        }
    , "")
}

fn extract_containers(resource) {
    let resources = [];
    for container in resource?.spec?.containers ?? [] {
        let meta = #{
            is_init: false
        };
        container.__meta = meta;
        resources.push(PseudoResource(container.name, container))
    }
    for container in resource?.spec?.initContainers ?? [] {
        let meta = #{
            is_init: true
        };
        container.__meta = meta;
        resources.push(PseudoResource(container.name, container))
    }
    resources
}


fn container_statuses(resource) {
    let status_map = #{};
    let pod_status = resource?.status;
    
    let statuses = (pod_status.containerStatuses ?? []) + (pod_status.initContainerStatuses ?? []);
    for container_status in statuses {
        status_map[container_status.name] = container_status;
    }
    status_map
}
