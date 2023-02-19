use cursive::reexports::log::error;
use cursive::{Cursive, View};

use crate::ui::traits::SivExt;
use crate::util::panics::ResultExt;

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
