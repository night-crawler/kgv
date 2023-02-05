use std::time::Instant;

use kube::api::GroupVersionKind;

pub mod code_view;
pub mod detail_view;
pub mod gvk_list_view;
pub mod gvk_switcher;
pub mod menu;
pub mod window_switcher;
pub mod log_view;

pub enum ViewType {
    ListView(GroupVersionKind, Instant),
    DetailView(String, Instant),
}
