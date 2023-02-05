import "commons" as commons;
import "pod" as pod;

fn name(resource) {
    resource?.name ?? "-"
}

fn image(resource) {
    resource?.image ?? "-"
}

fn is_init(resource) {
    commons::yes_no(resource?.__meta?.is_init)
}

fn restarts(resource, source) {
    if source == () || resource == () {
        debug("No source/resource provided");
        return "*E"
    }

    let status_map = pod::container_statuses(source);
    status_map[resource.name]?.restartCount ?? "-"
}

fn state(resource, source) {
    if source == () || resource == () {
        debug("No source/resource provided");
        return "*E"
    }

    let status = pod::container_statuses(source)[resource.name];
    if status == () {
        return "No status";
    }
    let state = status.state;
    if state == () {
        return "No state";
    }

    let terminated = state?.terminated;
    if terminated != () {
        return terminated.reason ?? "Terminated";
    }

    let waiting = state?.waiting;
    if waiting != () {
        return waiting.reason ?? "Waiting";
    }

    let running = state?.running;
    if running != () {
        return "Running";
    }

    return "Unknown";
}

fn mem_rl(resource) {
    let resources = resource?.resources;
    let limit = resources?.limits?.memory ?? "0";
    limit.replace("Mi", "");
    
    let request = resources?.requests?.memory ?? "0";
    request.replace("Mi", "");
    
    `${request}:${limit}`
}

fn cpu_rl(resource) {
    let resources = resource?.resources;
    let limit = resources?.limits?.cpu ?? "0";
    limit.replace("m", "");
    
    let request = resources?.requests?.cpu ?? "0";
    request.replace("m", "");
    
    `${request}:${limit}`
}


fn ports(resource) {
    commons::format_ports(resource?.ports)
}

fn probes(resource) {
    let liveness = if resource?.livenessProbe == () {
        "off"
    } else {
        "on"
    };
    let readiness = if resource?.readinessProbe == () {
        "off"
    } else {
        "on"
    };
    
    `${liveness}:${readiness}`
}


fn last_action(resource, source) {
    let status = pod::container_statuses(source)[resource.name];
    if status == () {
        return ""
    }

    let ts = status?.lastState?.terminated?.finishedAt;
    if ts == () {
        return ""
    }
    Result::Ago(ts)
}
