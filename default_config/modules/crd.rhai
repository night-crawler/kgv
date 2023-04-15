fn versions(resource) {
    let versions = resource?.spec?.versions ?? [];
    versions.map(|bundle| bundle.name).join(", ")
}

fn gvk(resource) {
    let versions = resource?.spec?.versions ?? [];
    let versions = versions.map(|bundle| bundle.name);

    let versions = if versions.len() > 1 {
        versions.join("|")
    } else if versions.len() == 1 {
        versions[0]
    } else {
        "-"
    };

    let group = resource?.spec?.group ?? "-";
    let kind = resource?.spec?.names?.kind ?? "-";

    `${group}/${versions}/${kind}`
}

fn names(resource) {
    let short = resource?.spec?.names?.shortNames;
    if short != () {
        short = short.join("|")
    }

    let plural = resource?.spec?.names?.plural;
    let singular = resource?.spec?.names?.singular;

    let parts = [
        short,
        singular,
        plural
    ];

    parts.filter(|part| part != "" && part != ()).join("/")
}
