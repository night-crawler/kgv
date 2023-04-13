fn yes_no(value) {
    if value == () {
        "-"
    } else if value == true {
        "yes"
    } else if value == false {
        "no"
    } else {
        "?"
    }
}

fn namespace(resource) {
    resource?.metadata?.namespace ?? ""
}

fn live_timestamp(ts) {
    if ts == () {
        "-"
    } else {
        Result::Ago(ts)
    }
}

fn age(resource) {
    Result::Ago(resource?.metadata?.creationTimestamp)
}

fn name(resource) { 
    resource?.metadata?.name ?? ""
}

fn is_deleted(resource) {
    resource?.metadata?.deletionTimestamp != ()
}

fn state(resource) {
    if cm::is_deleted(resource) {
        "Deleted"
    } else {
        "Active"
    }
}

fn compute_values_max_len(map) {
    let max_len = 0;
    for key in map.keys() {
        let value = `${map[key]}`;

        if value.len() > max_len {
            max_len = value.len();
        }
    }

    max_len
}

fn compute_keys_max_len(map) {
    let max_len = 0;
    for key in map.keys() {
        let value = `${key}`;

        if value.len() > max_len {
            max_len = value.len();
        }
    }

    max_len
}

fn format_port_mapping_full(port_mapping) {
    let name = port_mapping?.name;
    let port = port_mapping?.containerPort ?? port_mapping?.port;
    let protocol = port_mapping?.protocol;
    let target_port = port_mapping?.targetPort;

    if target_port != () && target_port != port {
        `${name}:${port}➤${target_port}/${protocol}`
    } else {
        `${name}:${port}/${protocol}`
    }
}

fn format_port_mapping_short(port_mapping) {
    let port = port_mapping?.containerPort ?? port_mapping?.port;
    let protocol = port_mapping?.protocol;
    `${port}/${protocol}`
}

fn format_ports(ports) {
    let ports = ports ?? [];
    let mappings = ports.map(format_port_mapping_full);
    let full_repr = mappings.join(",");

    if full_repr.len > 40 {
        ports.map(format_port_mapping_short).join(",");
    } else {
        full_repr
    }
}

fn will_table_fit_screen(map) {
    let a = compute_keys_max_len(map) + compute_values_max_len(map);
    a < 80
}

fn ready_conditions_count(conditions) {
    let conditions = conditions ?? [];
    let ready = 0;
    let count = conditions.len();

    for condition in conditions {
        let status = condition?.status ?? "";
        let status = `${status}`;

        if status.to_lower() == "true" {
            ready += 1;
        }
    }

    `${ready}/${count}`
}

fn display_map(map) {
    if map == () {
        return "";
    }

    let parts = [];

    for key in map.keys() {
        let value = map[key];
        value = if value.type_of() == "map" {
            display_map(value)
        } else if value.type_of() == "array" {
            value.join(",")
        } else {
            `${value}`
        };
        parts.push(`${key}:${value}`);
    }

    parts.join(",")
}
