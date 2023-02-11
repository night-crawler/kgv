use std::collections::BTreeMap;
use std::collections::HashMap;

use cruet::Inflector;
use kube::api::GroupVersionKind;
use lazy_static::lazy_static;

use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::util::k8s::gvk_sort_key;

pub mod impls;
pub mod traits;

lazy_static! {
    pub static ref GVK_TO_COLUMNS_MAP: HashMap<GroupVersionKind, Vec<ResourceColumn>> =
        ResourceView::build_gvk_to_columns_map();
}

pub fn group_gvks(gvks: Vec<GroupVersionKind>) -> Vec<(String, Vec<GroupVersionKind>)> {
    let mut misc = vec![];
    let mut map = BTreeMap::new();

    for gvk in gvks {
        let grouper = if gvk.kind == "CustomResourceDefinition" {
            "default"
        } else if gvk.kind.starts_with("PersistentVolume") {
            "storage"
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
