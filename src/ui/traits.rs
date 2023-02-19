use cursive::{Cursive, View};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;

pub trait MenuNameExt {
    fn full_menu_name(&self) -> String;
    fn short_menu_name(&self) -> String;
}

pub trait SivExt {
    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static;

    fn send_box<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static;
}

pub trait TableViewExt<T> {
    fn add_or_update_resource(&mut self, resource: T);
}

pub trait ContainerExt {
    fn memory_limit(&self) -> Option<&Quantity>;
    fn memory_request(&self) -> Option<&Quantity>;
    fn memory_rl(&self) -> String;

    fn cpu_limit(&self) -> Option<&Quantity>;
    fn cpu_request(&self) -> Option<&Quantity>;
    fn cpu_rl(&self) -> String;
}
