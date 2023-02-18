use kube::api::GroupVersionKind;

pub fn gvk_sort_key(gvk: &GroupVersionKind) -> (String, String, String) {
    (gvk.group.clone(), gvk.version.clone(), gvk.kind.clone())
}
