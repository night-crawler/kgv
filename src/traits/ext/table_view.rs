use cursive::reexports::log::error;
use cursive::Cursive;
use cursive_table_view::{TableView, TableViewItem};

use crate::traits::ext::cursive::CursiveTableExt;

pub trait TableViewExt<T> {
    fn add_or_update_resource(&mut self, resource: T);
}

pub trait TableCallBacks<T, H>
where
    T: TableViewItem<H> + Clone + 'static,
    H: Eq + std::hash::Hash + Copy + 'static,
{
    fn set_on_submit_named(&mut self, name: &str, callback: impl Fn(&mut Cursive, T) + 'static);

    fn set_on_select_named(&mut self, name: &str, callback: impl Fn(&mut Cursive, T) + 'static);
}

impl<T, H> TableCallBacks<T, H> for TableView<T, H>
where
    T: TableViewItem<H> + Clone + 'static,
    H: Eq + std::hash::Hash + Copy + 'static,
{
    //noinspection DuplicatedCode
    fn set_on_submit_named(&mut self, name: &str, callback: impl Fn(&mut Cursive, T) + 'static) {
        let name = name.to_string();
        self.set_on_submit(move |siv, _, index| {
            if let Some(item) = siv.get_table_item(&name, index) {
                callback(siv, item);
            } else {
                error!("Table did not have an item with index: {index}");
            }
        });
    }

    //noinspection DuplicatedCode
    fn set_on_select_named(&mut self, name: &str, callback: impl Fn(&mut Cursive, T) + 'static) {
        let name = name.to_string();
        self.set_on_select(move |siv, _, index| {
            if let Some(item) = siv.get_table_item(&name, index) {
                callback(siv, item);
            } else {
                error!("Table did not have an item with index: {index}");
            }
        });
    }
}
