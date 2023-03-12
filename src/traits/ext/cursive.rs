use std::path::PathBuf;

use anyhow::Context;
use cursive::reexports::log::error;
use cursive::{Cursive, View};
use cursive_table_view::{TableView, TableViewItem};

use crate::util::panics::ResultExt;

pub trait SivExt {
    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static;

    fn send_box<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static;
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
