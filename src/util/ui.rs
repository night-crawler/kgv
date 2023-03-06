use std::collections::BTreeMap;

use cruet::Inflector;
use cursive::traits::Nameable;
use cursive::views::{EditView, NamedView};
use cursive::Cursive;
use kube::api::GroupVersionKind;

use crate::util::k8s::gvk_sort_key;
use crate::util::panics::OptionExt;

pub fn ago(duration: chrono::Duration) -> String {
    if duration.num_nanoseconds() < Some(1000) {
        format!("{}ns", duration.num_nanoseconds().unwrap())
    } else if duration.num_microseconds() < Some(1000) {
        format!("{}μs", duration.num_microseconds().unwrap())
    } else if duration.num_milliseconds() < 1000 {
        format!("{}ms", duration.num_milliseconds())
    } else if duration.num_seconds() < 100 {
        format!("{}s", duration.num_seconds())
    } else if duration.num_minutes() < 100 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 100 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 100 {
        format!("{}d", duration.num_days())
    } else {
        format!("{}w", duration.num_weeks())
    }
}

pub fn group_gvks(gvks: Vec<GroupVersionKind>) -> Vec<(String, Vec<GroupVersionKind>)> {
    let mut misc = vec![];
    let mut map = BTreeMap::new();

    for gvk in gvks {
        let grouper = if gvk.kind == "CustomResourceDefinition" {
            "default"
        } else if gvk.kind.contains("PersistentVolume") {
            "storage"
        } else if gvk.group.contains("secret") {
            "Secret"
        } else if gvk.group.contains("istio") {
            "istio"
        } else if gvk.group.contains("api") {
            "API"
        } else if gvk.group.contains("apps") {
            "Default"
        } else if gvk.group.contains("flux") {
            "flux"
        } else if gvk.group.contains("monitoring")
            || gvk.group.contains("metric")
            || gvk.group.contains("telemetry")
        {
            "monitoring"
        } else if gvk.group.contains("acme") || gvk.group.contains("cert") {
            "Cert"
        } else if gvk.group.is_empty() {
            "default"
        } else if gvk.group.contains("admission") {
            "admission"
        } else if gvk.group.contains("flow") {
            "flow"
        } else {
            gvk.group.split('.').next().unwrap_or_log()
        }
        .to_title_case();
        map.entry(grouper).or_insert_with(Vec::new).push(gvk);
    }

    let default = map.remove("Default");
    let mut grouped = vec![];
    for (grouper, mut group) in map.into_iter() {
        if group.len() == 1 {
            misc.extend(group);
        } else {
            group.sort_unstable_by_key(gvk_sort_key);
            grouped.push((grouper, group));
        }
    }
    misc.sort_unstable_by_key(gvk_sort_key);

    grouped.sort_unstable_by_key(|(name, _)| name.clone());
    if let Some(group) = default {
        grouped.insert(0, ("Default".to_string(), group));
    }

    grouped.push(("Misc".to_string(), misc));
    grouped
}

pub fn build_edit_view<S, F>(name: S, initial: S, on_edit: F) -> NamedView<EditView>
where
    F: Fn(&mut Cursive, &str, usize) + 'static,
    S: Into<String>,
{
    EditView::new()
        .content(initial)
        .on_edit(on_edit)
        .with_name(name)
}
