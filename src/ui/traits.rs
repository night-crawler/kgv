use cursive::{Cursive, View};

use crate::model::resource_view::ResourceView;

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

pub trait TableViewExt {
    fn merge_resource(&mut self, resource: ResourceView);
}
