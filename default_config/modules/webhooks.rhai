fn extract_webhooks(resource) {
    let resources = [];
    for webhook in resource?.webhooks ?? [] {
        resources.push(PseudoResource(webhook.name, webhook))
    }
    resources
}
