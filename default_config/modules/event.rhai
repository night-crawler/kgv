fn gvk(resource) {
    let obj = resource?.involvedObject;
    if obj == () {
        return "-";
    }

    let parts = [obj.group, obj.apiVersion, obj.kind];
    let parts = parts.filter(|part| part != ());
    
    parts.join("/")
}

fn involved_name(resource) {
    let obj = resource?.involvedObject;
    if obj == () {
        return "-";
    }

    `${obj.namespace}/${obj.name}`
}
