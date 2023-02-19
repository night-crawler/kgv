use std::cmp::Ordering;

use crate::model::ext::bool_ext::BoolExt;
use crate::model::ext::container::ContainerExt;
use crate::model::ext::container_state::ContainerStateExt;
use crate::model::pod::pod_container_column::PodContainerColumn;
use crate::model::pod::pod_container_view::PodContainerView;
use cursive_table_view::{TableView, TableViewItem};

use crate::ui::traits::TableViewExt;
use crate::util::ui::ago;

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
            PodContainerColumn::Ready => self
                .status
                .as_ref()
                .map(|s| s.ready.as_yes_no().to_string())
                .unwrap_or_default(),
            PodContainerColumn::State => self.get_state_name().to_string(),
            PodContainerColumn::Init => self.is_init_container.as_yes_no().to_string(),
            PodContainerColumn::Restarts => self
                .status
                .as_ref()
                .map(|s| s.restart_count.to_string())
                .unwrap_or_default(),
            PodContainerColumn::Probes => format!(
                "{}:{}",
                self.container.liveness_probe.is_some().as_on_off(),
                self.container.readiness_probe.is_some().as_on_off(),
            ),
            PodContainerColumn::Cpu => self.container.cpu_rl(),
            PodContainerColumn::Mem => self.container.memory_rl(),
            PodContainerColumn::Ports => self.get_ports_repr(),
            PodContainerColumn::Age => self
                .status
                .as_ref()
                .and_then(|s| s.state.as_ref())
                .and_then(|s| s.get_age())
                .map(ago)
                .unwrap_or_default(),
        }
    }

    fn cmp(&self, other: &Self, column: PodContainerColumn) -> Ordering
    where
        Self: Sized,
    {
        self.to_column(column).cmp(&other.to_column(column))
    }
}
