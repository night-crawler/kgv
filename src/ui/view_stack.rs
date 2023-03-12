use std::ops::Deref;
use std::sync::{Arc, RwLock};

use cursive::reexports::ahash::HashMap;
use kube::api::GroupVersionKind;

use crate::ui::view_meta::ViewMeta;
use crate::util::panics::ResultExt;

#[derive(Default, Debug)]
pub struct ViewStack {
    pub stack: Vec<Arc<RwLock<ViewMeta>>>,
    pub map: HashMap<usize, Arc<RwLock<ViewMeta>>>,
}

impl ViewStack {
    pub fn push(&mut self, view: Arc<RwLock<ViewMeta>>) {
        let id = view.read().unwrap_or_log().get_id();
        self.stack.push(view.clone());
        self.map.insert(id, view);
    }

    pub fn find_all(&self, gvk: &GroupVersionKind) -> Vec<Arc<RwLock<ViewMeta>>> {
        self.stack
            .iter()
            .filter_map(|view| match view.read().unwrap_or_log().deref() {
                ViewMeta::List { gvk: list_gvk, .. } if list_gvk == gvk => Some(Arc::clone(view)),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub fn get(&mut self, view_id: usize) -> Option<Arc<RwLock<ViewMeta>>> {
        self.map.get(&view_id).map(Arc::clone)
    }

    pub fn pop(&mut self) {
        if let Some(view_meta) = self.stack.pop() {
            let id = view_meta.read().unwrap_or_log().get_id();
            self.map.remove(&id);
        }
    }
}
