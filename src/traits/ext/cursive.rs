use std::path::PathBuf;

use anyhow::Context;
use cursive::reexports::log::error;
use cursive::{Cursive, View};

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
