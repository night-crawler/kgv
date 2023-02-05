fn extract_external_ips(resource) {
    let type = resource?.spec?.type;

    if type == "ClusterIP" || type == "NodePort" {
        resource.spec.externalIPs ?? []
    } else if type == "ExternalName" {
        [resource.spec.externalName]
    } else if type == "LoadBalancer" {
        let externalIps = resource?.spec?.externalIPs ?? [];
        externalIps
    }
    
    else {
        ["todo"]
    }
}


fn external_ips(resource) {
    extract_external_ips(resource).join(",")
}
