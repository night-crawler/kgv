use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::Parser;
use cursive::reexports::log::{error, info, warn};
use cursive::{event, Cursive, CursiveRunnable};
use cursive_flexi_logger_view::toggle_flexi_logger_debug_console;
use k8s_openapi::api::core::v1::{Namespace, Pod};

use crate::backend::k8s_backend::K8sBackend;
use crate::config::args::Args;
use crate::config::extractor_configuration::{load_columns_config, load_embedded_columns_config};
use crate::config::kgv_configuration::KgvConfiguration;
use crate::eval::engine_factory::build_engine;
use crate::eval::evaluator::Evaluator;
use crate::theme::get_theme;
use crate::traits::ext::cursive::SivLogExt;
use crate::traits::ext::gvk::GvkStaticExt;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::dispatch::dispatch_events;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::{ResourceManager, UiStore};
use crate::util::panics::ResultExt;
use crate::util::paths::COLUMNS_FILE;
use crate::util::watcher::FlagWatcher;

pub mod backend;
pub mod config;
pub mod eval;
pub mod model;
pub mod theme;
pub mod traits;
pub mod ui;
pub mod util;

fn init_cursive_backend() -> std::io::Result<Box<dyn cursive::backend::Backend>> {
    let backend = cursive::backends::termion::Backend::init()?;
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(backend);
    Ok(Box::new(buffered_backend))
}

fn register_hotkeys(ui: &mut Cursive, ui_to_ui_sender: kanal::Sender<ToUiSignal>) {
    ui.add_global_callback('~', toggle_flexi_logger_debug_console);
    ui.add_global_callback(event::Key::F10, |siv| siv.select_menubar());
    ui.add_global_callback(event::Key::Esc, |siv| {
        siv.pop_layer();
    });
    {
        let ui_to_ui_sender = ui_to_ui_sender.clone();
        ui.add_global_callback(event::Event::CtrlChar('s'), move |_| {
            ui_to_ui_sender.send(ToUiSignal::CtrlSPressed).unwrap();
        });
    }
    ui.add_global_callback(event::Event::CtrlChar('y'), move |_| {
        ui_to_ui_sender.send(ToUiSignal::CtrlYPressed).unwrap();
    });
}

fn main() -> Result<()> {
    let kgv_configuration = KgvConfiguration::try_from(Args::parse())?;
    println!("{:?}", kgv_configuration);

    let mut ui = CursiveRunnable::default();
    ui.setup_logger(kgv_configuration.logs_dir)?;
    ui.set_theme(get_theme());

    let (from_client_sender, from_backend_receiver) = kanal::unbounded();
    let (to_backend_sender, from_ui_receiver) = kanal::unbounded();

    let ui_to_ui_sender = from_client_sender.clone();

    let extraction_config = match load_columns_config(COLUMNS_FILE.clone()) {
        Ok(config) => config,
        Err(err) => {
            warn!(
                "Could not read column config from {:?}: {}",
                COLUMNS_FILE.clone().into_os_string(),
                err
            );
            load_embedded_columns_config()?
        }
    };

    let mut backend = K8sBackend::new(
        from_client_sender,
        from_ui_receiver,
        kgv_configuration.cache_dir,
    )?;

    backend.spawn_watcher_exchange_task();
    backend.spawn_discovery_task();
    backend.spawn_from_ui_receiver_task();

    register_hotkeys(&mut ui, ui_to_ui_sender.clone());

    to_backend_sender
        .send(ToBackendSignal::RequestRegisterGvk(Pod::gvk_for_type()))
        .unwrap_or_log();
    to_backend_sender
        .send(ToBackendSignal::RequestRegisterGvk(
            Namespace::gvk_for_type(),
        ))
        .unwrap_or_log();
    ui_to_ui_sender
        .send(ToUiSignal::ShowGvk(Pod::gvk_for_type()))
        .unwrap_or_log();
    to_backend_sender
        .send(ToBackendSignal::RequestGvkItems(Pod::gvk_for_type()))
        .unwrap_or_log();

    let watcher = FlagWatcher::new(kgv_configuration.module_dirs, build_engine)?;

    let resource_manager = ResourceManager::new(
        Evaluator::new(4, watcher)?,
        ColumnRegistry::new(extraction_config.gvk_to_columns),
    );

    let store = Arc::new(Mutex::new(UiStore::new(
        ui.cb_sink().clone(),
        ui_to_ui_sender,
        to_backend_sender,
        resource_manager,
        ui::highlighter::Highlighter::new("base16-eighties.dark")?,
    )));

    dispatch_events(store.clone(), from_backend_receiver);

    loop {
        ui.try_run_with(init_cursive_backend)?;

        let mut store = store.lock().unwrap_or_log();
        if let Some(command) = store.interactive_command.take() {
            match command.run() {
                Ok(status) => {
                    if !status.success() {
                        error!("Failed to exec: {}", status);
                    } else {
                        info!("Executed: {}", status);
                    }
                }
                Err(err) => {
                    error!("Error executing command: {}", err);
                }
            }
        } else {
            break;
        }
    }

    Ok(())
}
