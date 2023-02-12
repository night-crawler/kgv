use std::cmp::Ordering;

use cursive_table_view::TableViewItem;
use itertools::Itertools;
use kube::api::GroupVersionKind;

use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::ui::traits::MenuNameExt;

impl TableViewItem<ResourceColumn> for ResourceView {
    fn to_column(&self, column: ResourceColumn) -> String {
        match column {
            ResourceColumn::Namespace => self.namespace(),
            ResourceColumn::Name => self.name(),
            ResourceColumn::Status => self.status(),
            ResourceColumn::Ready => self.ready(),
            ResourceColumn::Ip => self.ip(),
            ResourceColumn::Restarts => self.restarts(),
            ResourceColumn::Node => self.node(),
            ResourceColumn::Age => self.age(),
            
            _ => String::new(),
        }
    }

    fn cmp(&self, other: &Self, column: ResourceColumn) -> Ordering
    where
        Self: Sized,
    {
        self.to_column(column) .cmp(&other.to_column(column))
    }
}

impl MenuNameExt for GroupVersionKind {
    fn full_menu_name(&self) -> String {
        [&self.group, &self.version, &self.kind]
            .iter()
            .filter(|part| !part.is_empty())
            .join("/")
    }

    fn short_menu_name(&self) -> String {
        format!("{}/{}", &self.version, &self.kind)
    }
}
