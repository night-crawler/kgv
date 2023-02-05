use std::path::PathBuf;
use std::sync::Arc;

use crate::reexports::sync::RwLock;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::ui::view_meta::ViewMeta;
use anyhow::Context;
use cursive::reexports::log::{error, warn};
use cursive::{Cursive, View};
use cursive_table_view::{TableView, TableViewItem};

use crate::util::panics::ResultExt;

pub trait SivExt {
    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static;

    fn channel_call_on_name<V, F, R>(
        &self,
        sender: kanal::Sender<Box<dyn std::any::Any>>,
        name: &str,
        callback: F,
    ) where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static,
        R: 'static;

    fn send_box<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static;

    fn channel_box<F, R>(&self, sender: kanal::Sender<Box<dyn std::any::Any>>, callback: F)
    where
        F: FnOnce(&mut Cursive) -> R + Send + 'static,
        R: 'static;
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

    fn channel_call_on_name<V, F, R>(
        &self,
        sender: kanal::Sender<Box<dyn std::any::Any>>,
        name: &str,
        callback: F,
    ) where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static,
        R: 'static,
    {
        self.call_on_name(name, move |view| {
            let result = callback(view);
            sender.send_unwrap(Box::new(result));
        })
    }

    fn send_box<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static,
    {
        self.send(Box::new(callback)).unwrap_or_log();
    }

    fn channel_box<F, R>(&self, sender: kanal::Sender<Box<dyn std::any::Any>>, callback: F)
    where
        F: FnOnce(&mut Cursive) -> R + Send + 'static,
        R: 'static,
    {
        self.send_box(move |siv| {
            let result = callback(siv);
            sender.send_unwrap(Box::new(result));
        })
    }
}

pub trait SivLogExt {
    fn setup_logger(&mut self, logs_dir: Option<PathBuf>) -> anyhow::Result<()>;
}

impl SivLogExt for Cursive {
    fn setup_logger(&mut self, logs_dir: Option<PathBuf>) -> anyhow::Result<()> {
        let logs_dir = logs_dir.context("No logs dir")?;

        flexi_logger::Logger::try_with_env_or_str("info")?
            .log_to_file_and_writer(
                flexi_logger::FileSpec::default()
                    .directory(logs_dir)
                    .suppress_timestamp(),
                cursive_flexi_logger_view::cursive_flexi_logger(self),
            )
            .format(flexi_logger::colored_with_thread)
            .start()?;

        Ok(())
    }
}

pub trait CursiveTableExt<T, H>
where
    T: TableViewItem<H> + Clone + 'static,
    H: Eq + std::hash::Hash + Copy + 'static,
{
    fn get_table_item(&mut self, name: &str, index: usize) -> Option<T>;
}

impl<T, H> CursiveTableExt<T, H> for Cursive
where
    T: TableViewItem<H> + Clone + 'static,
    H: Eq + std::hash::Hash + Copy + 'static,
{
    fn get_table_item(&mut self, name: &str, index: usize) -> Option<T> {
        let result = self.call_on_name(name, |table: &mut TableView<T, H>| {
            table.borrow_item(index).cloned()
        });

        if let Some(call_result) = result {
            call_result
        } else {
            error!("Could not find a table with name: {name}");
            None
        }
    }
}

pub trait SivUtilExt {
    fn remove_views(&mut self, view_meta_list: Vec<Arc<RwLock<ViewMeta>>>);
}

impl SivUtilExt for Cursive {
    fn remove_views(&mut self, view_meta_list: Vec<Arc<RwLock<ViewMeta>>>) {
        for meta in view_meta_list.iter() {
            let name = meta.read_unwrap().get_unique_name();
            if let Some(pos) = self.screen_mut().find_layer_from_name(&name) {
                warn!("Removing layer with name {}", name);
                self.screen_mut().remove_layer(pos);
            } else {
                error!("Could not find layer with name: {}", name);
            }
        }
    }
}
