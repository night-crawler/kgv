use std::ops::Deref;
use std::sync::Arc;

use anyhow::Context;
use cursive::reexports::ahash::HashMap;
use kube::api::GroupVersionKind;

use crate::reexports::sync::RwLock;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::ui::view_meta::ViewMeta;

#[derive(Default, Debug)]
pub(crate) struct ViewStack {
    pub(crate) stack: Vec<Arc<RwLock<ViewMeta>>>,
    pub(crate) view_by_id_map: HashMap<usize, Arc<RwLock<ViewMeta>>>,
}

impl ViewStack {
    pub(crate) fn push(&mut self, view: Arc<RwLock<ViewMeta>>) {
        let id = view.read_unwrap().get_id();
        self.stack.push(view.clone());
        self.view_by_id_map.insert(id, view);
    }

    pub(crate) fn find_logs(&self) -> Vec<Arc<RwLock<ViewMeta>>> {
        self.stack
            .iter()
            .filter(|view| matches!(view.read_unwrap().deref(), ViewMeta::Logs { .. }))
            .cloned()
            .collect()
    }

    pub(crate) fn find_all_by_gvk(&self, gvk: &GroupVersionKind) -> Vec<Arc<RwLock<ViewMeta>>> {
        self.stack
            .iter()
            .filter_map(|view| match view.read_unwrap().deref() {
                ViewMeta::List { gvk: self_gvk, .. }
                | ViewMeta::Details { gvk: self_gvk, .. }
                | ViewMeta::Code { gvk: self_gvk, .. }
                    if self_gvk == gvk =>
                {
                    Some(Arc::clone(view))
                }
                _ => None,
            })
            .collect()
    }

    pub(crate) fn find_window_switchers(&self) -> Vec<Arc<RwLock<ViewMeta>>> {
        self.stack
            .iter()
            .filter(|view| matches!(view.read_unwrap().deref(), ViewMeta::WindowSwitcher { .. }))
            .cloned()
            .collect()
    }

    pub(crate) fn find_gvk_switchers(&self) -> Vec<Arc<RwLock<ViewMeta>>> {
        self.stack
            .iter()
            .filter(|view| matches!(view.read_unwrap().deref(), ViewMeta::GvkSwitcher { .. }))
            .cloned()
            .collect()
    }

    pub(crate) fn remove_window_switchers(&mut self) {
        self.stack
            .retain(|meta| !matches!(meta.read_unwrap().deref(), ViewMeta::WindowSwitcher { .. }));
    }

    pub(crate) fn remove_gvk_switchers(&mut self) {
        self.stack
            .retain(|meta| !matches!(meta.read_unwrap().deref(), ViewMeta::GvkSwitcher { .. }));
    }

    pub(crate) fn get(&self, view_id: usize) -> Option<Arc<RwLock<ViewMeta>>> {
        self.view_by_id_map.get(&view_id).map(Arc::clone)
    }

    pub(crate) fn move_to_front(&mut self, view_id: usize) -> anyhow::Result<()> {
        let pos = self
            .stack
            .iter()
            .position(|item| item.read_unwrap().get_id() == view_id)
            .with_context(|| format!("View with id={view_id} was not found"))?;
        let last = self.stack.len() - 1;
        self.stack.swap(pos, last);
        Ok(())
    }

    pub(crate) fn pop(&mut self) {
        if let Some(view_meta) = self.stack.pop() {
            let id = view_meta.read_unwrap().get_id();
            self.view_by_id_map.remove(&id);
        }
    }

    pub(crate) fn last(&self) -> Option<Arc<RwLock<ViewMeta>>> {
        self.stack.last().map(Arc::clone)
    }
}
