use std::sync::{Arc, Mutex};

use anyhow::Result;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::Info;
use cursive::{event, CursiveRunnable};

use crate::model::resource_column::ResourceColumn;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::k8s_backend::K8sBackend;
use crate::ui::signals::ToUiSignal;
use crate::ui::ui_store::{UiStore, UiStoreExt};

pub mod model;
pub mod theme;
pub mod ui;
pub mod util;

fn main() -> Result<()> {
    cursive::logger::init();
    log::set_max_level(Info);

    let (from_client_sender, from_client_receiver) = kanal::unbounded();
    let (to_backend_sender, from_ui_receiver) = kanal::unbounded();

    let ui_to_ui_sender = from_client_sender.clone();

    let mut backend = K8sBackend::new(from_client_sender, from_ui_receiver)?;

    backend.spawn_watcher_exchange_task();
    backend.spawn_discovery_task();
    backend.spawn_from_ui_receiver_task();

    let mut ui = CursiveRunnable::default();
    ui.add_global_callback('~', |siv| siv.toggle_debug_console());
    ui.add_global_callback(event::Key::F10, |siv| siv.select_menubar());
    ui.add_global_callback(event::Key::Esc, |siv| {
        siv.pop_layer();
    });

    let store = Arc::new(Mutex::new(UiStore::new(
        ui.cb_sink().clone(),
        ui_to_ui_sender,
        to_backend_sender,
        ColumnRegistry::default(),
    )));

    std::thread::spawn(move || {
        let store = Arc::clone(&store);
        for signal in from_client_receiver {
            match signal {
                ToUiSignal::ResponseResourceUpdated(resource) => {
                    store.handle_response_resource_updated(resource);
                }
                ToUiSignal::ResponseDiscoveredGvks(gvks) => {
                    store.handle_response_discovered_gvks(gvks);
                }
                ToUiSignal::ResponseGvkItems(next_gvk, resources) => {
                    store.handle_response_gvk_items(next_gvk, resources);
                }
                ToUiSignal::ApplyNamespaceFilter(ns) => {
                    store.handle_apply_namespace_filter(ns);
                }
                ToUiSignal::ApplyNameFilter(name) => {
                    store.handle_apply_name_filter(name);
                }
                ToUiSignal::ShowDetails(resource) => {
                    store.handle_show_details(resource);
                }
            }
        }
    });

    let backend_init = || -> std::io::Result<Box<dyn cursive::backend::Backend>> {
        let backend = cursive::backends::termion::Backend::init()?;
        let buffered_backend = cursive_buffered_backend::BufferedBackend::new(backend);
        Ok(Box::new(buffered_backend))
    };

    ui.try_run_with(backend_init)?;

    Ok(())
}
