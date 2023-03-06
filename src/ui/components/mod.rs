use kube::api::GroupVersionKind;
use std::time::Instant;

pub mod code_view;
pub mod detail_view;
pub mod gvk_list_view;
pub mod menu;
pub mod pod_detail;

pub enum ViewType {
    ListView(GroupVersionKind, Instant),
    DetailView(String, Instant),
}
