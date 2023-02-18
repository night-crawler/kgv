use std::collections::BTreeMap;

use cruet::Inflector;
use kube::api::GroupVersionKind;

use crate::util::k8s::gvk_sort_key;

pub mod column_registry;
pub mod components;
pub mod fs_cache;
pub mod impls;
pub mod k8s_backend;
pub mod signals;
pub mod traits;
pub mod ui_store;

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
            gvk.group.split('.').next().unwrap()
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
