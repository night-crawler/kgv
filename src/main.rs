use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::backend::k8s_backend::K8sBackend;
use anyhow::Result;
use cursive::reexports::crossbeam_channel::internal::SelectHandle;
use cursive::reexports::log;
use cursive::reexports::log::warn;
use cursive::{event, Cursive, CursiveRunnable};
use cursive_flexi_logger_view::toggle_flexi_logger_debug_console;
use k8s_openapi::api::core::v1::{Namespace, Pod};

use crate::model::traits::GvkStaticExt;
use crate::theme::get_theme;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::util::panics::{OptionExt, ResultExt};

pub mod backend;
pub mod model;
pub mod theme;
pub mod ui;
pub mod util;

fn setup_logging(siv: &Cursive) {
    let home = home::home_dir().unwrap_or_log().join(".kgv").join("logs");
    flexi_logger::Logger::try_with_env_or_str("info")
        .expect("Could not create Logger from environment :(")
        .log_to_file_and_writer(
            flexi_logger::FileSpec::default()
                .directory(home)
                .suppress_timestamp(),
            cursive_flexi_logger_view::cursive_flexi_logger(siv),
        )
        .format(flexi_logger::colored_with_thread)
        .start()
        .expect("failed to initialize logger!");
}

fn dispatch_events(store: Arc<Mutex<UiStore>>, from_backend_receiver: kanal::Receiver<ToUiSignal>) {
    std::thread::Builder::new()
        .name("dispatcher".to_string())
        .spawn(move || {
            for signal in from_backend_receiver {
                while !store.lock().unwrap_or_log().sink.is_ready() {
                    warn!("UI is not ready");
                    std::thread::sleep(Duration::from_millis(50));
                }

                match signal {
                    ToUiSignal::ResponseResourceUpdated(resource) => {
                        store.dispatch_response_resource_updated(resource);
                    }
                    ToUiSignal::ResponseDiscoveredGvks(gvks) => {
                        store.dispatch_response_discovered_gvks(gvks);
                    }
                    ToUiSignal::ResponseGvkItems(next_gvk, resources) => {
                        store.dispatch_response_gvk_items(next_gvk, resources);
                    }
                    ToUiSignal::ApplyNamespaceFilter(ns) => {
                        store.dispatch_apply_namespace_filter(ns);
                    }
                    ToUiSignal::ApplyNameFilter(name) => {
                        store.dispatch_apply_name_filter(name);
                    }
                    ToUiSignal::ShowDetails(resource) => {
                        store.dispatch_show_details(resource);
                    }
                    ToUiSignal::ShowGvk(gvk) => {
                        store.dispatch_show_gvk(gvk);
                    }
                }
            }
        })
        .unwrap_or_log();
}

fn backend_init() -> std::io::Result<Box<dyn cursive::backend::Backend>> {
    let backend = cursive::backends::termion::Backend::init()?;
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(backend);
    Ok(Box::new(buffered_backend))
}

fn main() -> Result<()> {
    let (from_client_sender, from_backend_receiver) = kanal::unbounded();
    let (to_backend_sender, from_ui_receiver) = kanal::unbounded();

    let ui_to_ui_sender = from_client_sender.clone();

    let mut ui = CursiveRunnable::default();
    ui.set_theme(get_theme());
    setup_logging(&ui);

    let mut backend = K8sBackend::new(from_client_sender, from_ui_receiver)?;

    backend.spawn_watcher_exchange_task();
    backend.spawn_discovery_task();
    backend.spawn_from_ui_receiver_task();

    ui.add_global_callback('~', toggle_flexi_logger_debug_console); // Bind '~' key to show/hide debug console view

    ui.add_global_callback(event::Key::F10, |siv| siv.select_menubar());
    ui.add_global_callback(event::Key::Esc, |siv| {
        siv.pop_layer();
    });

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

    let store = Arc::new(Mutex::new(UiStore::new(
        ui.cb_sink().clone(),
        ui_to_ui_sender,
        to_backend_sender,
        ColumnRegistry::default(),
    )));

    dispatch_events(store.clone(), from_backend_receiver);

    loop {
        ui.try_run_with(backend_init)?;

        let mut store = store.lock().unwrap_or_log();
        if let Some(command) = store.interactive_command.take() {
            log::info!("Handling command: {:?}", command);
            Command::new("mcedit")
                .args(&["--nosubshell", "/tmp/sample"])
                .spawn()
                .unwrap_or_log()
                .wait()
                .unwrap_or_log();
        } else {
            break;
        }
    }

    Ok(())
}
