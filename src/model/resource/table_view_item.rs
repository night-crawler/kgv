use crate::model::resource::resource_column::ResourceColumn;
use crate::model::resource::resource_view::ResourceView;
use crate::ui::traits::TableViewExt;
use crate::util::ui::ago;
use cursive_table_view::{TableView, TableViewItem};
use std::cmp::Ordering;

impl TableViewItem<ResourceColumn> for ResourceView {
    fn to_column(&self, column: ResourceColumn) -> String {
        match column {
            ResourceColumn::Namespace => self.namespace(),
            ResourceColumn::Name => self.name(),
            ResourceColumn::Status => self.status(),
            ResourceColumn::Ready => self.ready(),
            ResourceColumn::Ip => self.ips().map(|ips| ips.join(", ")).unwrap_or_default(),
            ResourceColumn::Restarts => self.restarts(),
            ResourceColumn::Node => self.node(),
            ResourceColumn::Age => ago(self.age()),

            _ => String::new(),
        }
    }

    fn cmp(&self, other: &Self, column: ResourceColumn) -> Ordering
    where
        Self: Sized,
    {
        self.to_column(column).cmp(&other.to_column(column))
    }
}

impl TableViewExt<ResourceView> for TableView<ResourceView, ResourceColumn> {
    fn add_or_update_resource(&mut self, resource: ResourceView) {
        for item in self.borrow_items_mut() {
            if item.uid() == resource.uid() {
                *item = resource;
                return;
            }
        }
        self.insert_item(resource);
    }
}
