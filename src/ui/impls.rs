use std::cmp::Ordering;
use cursive_table_view::TableViewItem;
use kube::api::GroupVersionKind;
use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::ui::traits::MenuNameExt;

impl TableViewItem<ResourceColumn> for ResourceView {
    fn to_column(&self, column: ResourceColumn) -> String {
        match column {
            ResourceColumn::Namespace => self.namespace(),
            ResourceColumn::Name => self.name(),
            _ => String::new(),
        }
    }

    fn cmp(&self, other: &Self, column: ResourceColumn) -> Ordering
        where
            Self: Sized,
    {
        match column {
            ResourceColumn::Name => self.name().cmp(&other.name()),
            ResourceColumn::Namespace => self.namespace().cmp(&other.namespace()),
            _ => Ordering::Equal,
        }
    }
}

impl MenuNameExt for GroupVersionKind {
    fn full_menu_name(&self) -> String {
        format!("{}/{}/{}", &self.group, &self.version, &self.kind)
    }

    fn short_menu_name(&self) -> String {
        format!("{}/{}", &self.version, &self.kind)
    }
}