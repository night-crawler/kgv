use std::sync::{Arc, Mutex};

use anyhow::Result;
use cursive::reexports::log::{error, info};
use cursive::{event, Cursive, CursiveRunnable};
use cursive_flexi_logger_view::toggle_flexi_logger_debug_console;
use k8s_openapi::api::core::v1::{Namespace, Pod};

use crate::backend::k8s_backend::K8sBackend;
use crate::model::traits::GvkStaticExt;
use crate::theme::get_theme;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::dispatch::dispatch_events;
use crate::ui::logging::setup_logging;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::UiStore;
use crate::util::panics::ResultExt;

pub mod backend;
pub mod model;
pub mod theme;
pub mod ui;
pub mod util;

fn backend_init() -> std::io::Result<Box<dyn cursive::backend::Backend>> {
    let backend = cursive::backends::termion::Backend::init()?;
    let buffered_backend = cursive_buffered_backend::BufferedBackend::new(backend);
    Ok(Box::new(buffered_backend))
}

fn register_hotkeys(ui: &mut Cursive, ui_to_ui_sender: kanal::Sender<ToUiSignal>) {
    ui.add_global_callback('~', toggle_flexi_logger_debug_console); // Bind '~' key to show/hide debug console view

    ui.add_global_callback(event::Key::F10, |siv| siv.select_menubar());
    ui.add_global_callback(event::Key::Esc, |siv| {
        siv.pop_layer();
    });
    ui.add_global_callback(event::Event::CtrlChar('e'), move |_| {
        ui_to_ui_sender.send(ToUiSignal::CtrlEPressed).unwrap();
    });
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
