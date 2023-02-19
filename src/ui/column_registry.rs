use std::collections::HashMap;

use crate::model::resource::resource_column::ResourceColumn;
use crate::model::resource::resource_view::ResourceView;
use cursive::reexports::log::info;
use kube::api::GroupVersionKind;

pub struct ColumnRegistry {
    map: HashMap<GroupVersionKind, Vec<ResourceColumn>>,
}

impl Default for ColumnRegistry {
    fn default() -> Self {
        Self {
            map: ResourceView::build_gvk_to_columns_map(),
        }
    }
}

impl ColumnRegistry {
    pub fn get_columns(&self, gvk: &GroupVersionKind) -> Vec<ResourceColumn> {
        if let Some(columns) = self.map.get(gvk) {
            columns.to_vec()
        } else {
            info!("Columns for GVK {:?} were not found", gvk);
            vec![
                ResourceColumn::Namespace,
                ResourceColumn::Name,
                ResourceColumn::Status,
                ResourceColumn::Age,
            ]
        }
    }
}
