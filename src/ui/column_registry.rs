use std::collections::HashMap;

use kube::api::GroupVersionKind;

use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;

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
            vec![ResourceColumn::Namespace, ResourceColumn::Name]
        }
    }
}
