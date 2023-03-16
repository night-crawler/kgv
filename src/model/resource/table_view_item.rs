use std::cmp::Ordering;

use cursive::reexports::log::info;
use cursive_table_view::{TableView, TableViewItem};

use crate::model::resource::resource_view::EvaluatedResource;
use crate::traits::ext::table_view::TableViewExt;

impl TableViewItem<usize> for EvaluatedResource {
    fn to_column(&self, column: usize) -> String {
        self.values[column].to_string()
    }

    fn cmp(&self, other: &Self, column: usize) -> Ordering
    where
        Self: Sized,
    {
        self.values[column].cmp(&other.values[column])
    }
}

impl TableViewExt<EvaluatedResource> for TableView<EvaluatedResource, usize> {
    fn add_or_update_resource(&mut self, evaluated_resource: EvaluatedResource) {
        for item in self.borrow_items_mut() {
            if item.resource.uid() == evaluated_resource.resource.uid() {
                info!("Updated by uid {}", item.resource.full_unique_name());
                *item = evaluated_resource;
                return;
            }
        }
        info!(
            "Inserting a new table item: {}",
            evaluated_resource.resource.full_unique_name()
        );
        self.insert_item(evaluated_resource);
    }
}
