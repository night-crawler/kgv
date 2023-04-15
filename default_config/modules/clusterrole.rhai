const VERBS = ["get", "list", "watch", "create", "patch", "update", "delete", "deletecollection", "approve", "bind", "escalate", "proxy", "redirect", "connect",];

fn repr_api_groups(rule) {
    rule.apiGroups
        .map(|group| if group.is_empty() {
            "''"
        } else {
            group
        }
    ).join("|")
}

fn extract_verb_map(rule) {
    let verbs = rule.verbs ?? [];
    let verb_map = #{};

    if verbs.contains("*") {
        for verb in global::VERBS {
            verb_map[verb] = true;
        }
        return verb_map;
    }

    for verb in global::VERBS {
        verb_map[verb] = verbs.contains(verb);
    }
    verb_map
}

fn extract_rule_rule_name_product(rule) {
    let joined_api_groups = repr_api_groups(rule);;

    let processed = [];

    let verb_map = extract_verb_map(rule);

    for resource in rule.resources {
        for resource_name in rule.resourceNames {
            let extracted = #{
                rule: rule,
                api_groups: joined_api_groups,
                resource: resource,
                resource_name: resource_name,
            };
            extracted += verb_map;
            processed.push(extracted);
        }
    }

    processed
}

fn extract_urls(rule) {
    let processed = [];
    let verb_map = extract_verb_map(rule);
    for url in rule.nonResourceURLs {
        let extracted = #{
            rule: rule,
            url: url,
        };
        extracted += verb_map;
        processed.push(extracted);
    }

    processed
}

fn extract_default_rule(rule) {
    let processed = [];
    let verb_map = extract_verb_map(rule);
    let joined_api_groups = repr_api_groups(rule);

    for resource in rule.resources {
        let extracted = #{
            rule: rule,
            resource: resource,
            api_groups: joined_api_groups
        };
        extracted += verb_map;
        processed.push(extracted);
    }

    processed
}

fn extract_rules(resource) {
    let rules = resource?.rules ?? [];
    let processed = [];
    for rule in rules {
        if rule.apiGroups == () {
            processed += extract_urls(rule);
            continue;
        }

        if rule.resources == () {
            continue;
        }

        if rule.resourceNames != () {
            processed += extract_rule_rule_name_product(rule);
            continue;
        }

        processed += extract_default_rule(rule);
    }

    let resources = [];

    let counter = 0;
    for rule in processed {
        let name_parts = [
            counter,
            rule.api_groups,
            rule.resource,
            rule.resource_name,
            rule.url,
        ];
        resources.push(PseudoResource(name_parts.join(":"), rule));
        counter += 1;
    }

    resources
}

fn repr_bool(value) {
    if value == () {
        return "?"
    }

    if value {
        "✔"
    } else {
        ""
    }
}

fn url_or_resource(resource) {
    if resource.url != () {
        `URL: ${resource.url}`
    } else {
        resource.resource
    }
}