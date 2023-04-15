fn traffic_policy(policy_map) {
    if policy_map == () {
        return "-"
    }
    let parts = [];

    for policy_name in policy_map.keys() {
        let policy = policy_map[policy_name] ?? #{};

        for policy_key in policy.keys() {
            let policy_value = policy[policy_key];
            parts.push(`${policy_name}:${policy_key}:${policy_value}`);
        }
    }

    parts.join(",")
}

fn count_hosts(resource) {
    let count = 0;
    let servers = resource?.spec?.servers ?? [];

    for server in servers {
        count += (server?.hosts ?? []).len();
    }

    count
}

fn egress_hosts(resource) {
    let egress_items = resource?.spec?.egress ?? [];
    let joined_egress_parts = [];
    for egress in egress_items {
        let hosts = egress?.hosts ?? [];
        let joined_hosts = hosts.join(",");
        joined_egress_parts.push(joined_hosts);
    }

    joined_egress_parts.join("; ")
}
