use std::ops::Deref;
use std::sync::Arc;

use crate::reexports::RwLock;
use cursive::reexports::ahash::HashMap;
use kube::api::GroupVersionKind;

use crate::traits::ext::rw_lock::RwLockExt;
use crate::ui::view_meta::ViewMeta;

#[derive(Default, Debug)]
pub struct ViewStack {
    pub stack: Vec<Arc<RwLock<ViewMeta>>>,
    pub view_by_id_map: HashMap<usize, Arc<RwLock<ViewMeta>>>,
}

impl ViewStack {
    pub fn push(&mut self, view: Arc<RwLock<ViewMeta>>) {
        let id = view.read_unwrap().get_id();
        self.stack.push(view.clone());
        self.view_by_id_map.insert(id, view);
    }

    pub fn find_all(&self, gvk: &GroupVersionKind) -> Vec<Arc<RwLock<ViewMeta>>> {
        self.stack
            .iter()
            .filter_map(|view| match view.read_unwrap().deref() {
                ViewMeta::List { gvk: list_gvk, .. } if list_gvk == gvk => Some(Arc::clone(view)),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub fn get(&mut self, view_id: usize) -> Option<Arc<RwLock<ViewMeta>>> {
        self.view_by_id_map.get(&view_id).map(Arc::clone)
    }

    pub fn pop(&mut self) {
        if let Some(view_meta) = self.stack.pop() {
            let id = view_meta.read_unwrap().get_id();
            self.view_by_id_map.remove(&id);
        }
    }

    pub fn last(&self) -> Option<Arc<RwLock<ViewMeta>>> {
        self.stack.last().map(Arc::clone)
    }
}
