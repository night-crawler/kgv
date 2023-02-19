use crate::ui::pod::pod_container_column::PodContainerColumn;
use crate::ui::pod::pod_container_view::PodContainerView;
use crate::ui::traits::{ContainerExt, TableViewExt};
use cursive_table_view::{TableView, TableViewItem};
use std::cmp::Ordering;

impl TableViewExt<PodContainerView> for TableView<PodContainerView, PodContainerColumn> {
    fn add_or_update_resource(&mut self, resource: PodContainerView) {
        for item in self.borrow_items_mut() {
            if item.container.name == resource.container.name {
                *item = resource;
                return;
            }
        }
        self.insert_item(resource);
    }
}

impl TableViewItem<PodContainerColumn> for PodContainerView {
    fn to_column(&self, column: PodContainerColumn) -> String {
        match column {
            PodContainerColumn::Name => self.container.name.clone(),
            PodContainerColumn::Image => self.container.image.clone().unwrap_or_default(),
            PodContainerColumn::Ready => "".to_string(),
            PodContainerColumn::State => "".to_string(),
            PodContainerColumn::Init => "".to_string(),
            PodContainerColumn::Restarts => "".to_string(),
            PodContainerColumn::Probes => "".to_string(),
            PodContainerColumn::Cpu => self.container.cpu_rl(),
            PodContainerColumn::Mem => self.container.memory_rl(),
            PodContainerColumn::Ports => "".to_string(),
            PodContainerColumn::Age => "".to_string(),
        }
    }

    fn cmp(&self, other: &Self, column: PodContainerColumn) -> Ordering
    where
        Self: Sized,
    {
        self.to_column(column).cmp(&other.to_column(column))
    }
}
