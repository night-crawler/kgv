use std::cmp::Ordering;

use cursive::reexports::log::error;
use cursive::{Cursive, View};
use cursive_table_view::{TableView, TableViewItem};
use itertools::Itertools;
use k8s_openapi::api::core::v1::Container;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use kube::api::GroupVersionKind;

use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::ui::traits::{ContainerExt, MenuNameExt, SivExt, TableViewExt};
use crate::util::panics::ResultExt;
use crate::util::ui::ago;

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

impl SivExt
    for cursive::reexports::crossbeam_channel::Sender<Box<dyn FnOnce(&mut Cursive) + Send>>
{
    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static,
    {
        let name = name.to_string();
        self.send(Box::new(move |siv| {
            if siv.call_on_name(&name, callback).is_none() {
                error!("Could not find name: {}", name);
            }
        }))
        .unwrap_or_log();
    }

    fn send_box<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static,
    {
        self.send(Box::new(callback)).unwrap_or_log();
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

impl ContainerExt for Container {
    fn memory_limit(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.limits.as_ref()?.get("memory")
    }

    fn memory_request(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.requests.as_ref()?.get("memory")
    }

    fn memory_rl(&self) -> String {
        let request = self.memory_request().map(|q| q.0.as_str()).unwrap_or("-");
        let limit = self.memory_limit().map(|q| q.0.as_str()).unwrap_or("-");
        format!("{request}:{limit}")
    }

    fn cpu_limit(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.limits.as_ref()?.get("cpu")
    }

    fn cpu_request(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.requests.as_ref()?.get("cpu")
    }

    fn cpu_rl(&self) -> String {
        let request = self.cpu_request().map(|q| q.0.as_str()).unwrap_or("-");
        let limit = self.cpu_limit().map(|q| q.0.as_str()).unwrap_or("-");
        format!("{request}:{limit}")
    }
}
