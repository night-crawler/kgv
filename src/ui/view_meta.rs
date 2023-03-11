use kube::api::GroupVersionKind;

use crate::traits::ext::gvk::GvkNameExt;

#[derive(Debug, Default, Clone, Hash)]
pub struct Filter {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Hash)]
pub enum ViewMeta {
    List {
        id: usize,
        gvk: GroupVersionKind,
        filter: Filter,
    },
    Detail {
        id: usize,
        gvk: GroupVersionKind,
        uid: String,
    },
    PodDetail {
        id: usize,
        uid: String,
    },
    Dialog {
        id: usize,
        name: String,
    },
}

impl ViewMeta {
    pub fn get_unique_name(&self) -> String {
        match self {
            ViewMeta::List { id, gvk, filter: _ } => {
                format!("gvk-list-{id}-{}-table", gvk.full_name())
            }
            ViewMeta::Detail { id, gvk, uid } => {
                format!("gvk-details-{id}-{}-{uid}", gvk.full_name())
            }
            ViewMeta::PodDetail { id, uid } => format!("pod-detail-{id}-{uid}"),
            ViewMeta::Dialog { id, name } => format!("dialog-{id}-{name}"),
        }
    }

    pub fn get_edit_name(&self, edit_type: &str) -> String {
        format!("{}-{edit_type}", self.get_unique_name())
    }

    pub fn get_panel_name(&self) -> String {
        format!("{}-panel", self.get_unique_name())
    }

    pub fn set_namespace(&mut self, namespace: String) {
        match self {
            ViewMeta::List { filter, .. } => filter.namespace = namespace,
            this => panic!("Setting namespace {namespace} on {:?}", this),
        }
    }

    pub fn set_name(&mut self, name: String) {
        match self {
            ViewMeta::List { filter, .. } => filter.name = name,
            this => panic!("Setting namespace {name} on {:?}", this),
        }
    }

    pub fn get_id(&self) -> usize {
        match self {
            ViewMeta::List { id, .. }
            | ViewMeta::Detail { id, .. }
            | ViewMeta::PodDetail { id, .. }
            | ViewMeta::Dialog { id, .. } => *id,
        }
    }

    pub fn get_filter(&self) -> &Filter {
        match self {
            ViewMeta::List { filter, .. } => filter,
            this => panic!("Trying to get filter on {:?}", this),
        }
    }

    pub fn get_gvk(&self) -> &GroupVersionKind {
        match self {
            ViewMeta::List { gvk, .. } | ViewMeta::Detail { gvk, .. } => gvk,
            this => panic!("{:?} does not have GVK", this),
        }
    }
}
