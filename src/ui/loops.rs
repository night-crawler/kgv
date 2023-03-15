use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

use cursive::reexports::log::{error, info};
use cursive::CursiveRunnable;

use crate::traits::ext::mutex::MutexExt;
use crate::ui::backend::init_cursive_backend;
use crate::ui::signals::ToUiSignal;
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::util::panics::ResultExt;

pub fn spawn_dispatch_events_loop(
    store: Arc<Mutex<UiStore>>,
    from_backend_receiver: kanal::Receiver<ToUiSignal>,
) {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    std::thread::Builder::new()
        .name(format!(
            "dispatcher-{}",
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
        .spawn(move || {
            for signal in from_backend_receiver {
                let now = std::time::Instant::now();
                let signal_name = signal.as_ref().to_string();

                info!("Dispatching signal: {signal_name}");
                dispatch_signal(&store, signal);
                info!("Dispatching {signal_name} took {:?}", now.elapsed());
            }
        })
        .unwrap_or_log();
}

fn dispatch_signal(store: &Arc<Mutex<UiStore>>, signal: ToUiSignal) {
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
        ToUiSignal::ApplyNamespaceFilter(id, ns) => {
            store.dispatch_apply_namespace_filter(id, ns);
        }
        ToUiSignal::ApplyNameFilter(id, name) => {
            store.dispatch_apply_name_filter(id, name);
        }
        ToUiSignal::ShowDetails(resource) => {
            store.dispatch_show_details(resource);
        }
        ToUiSignal::ShowGvk(gvk) => {
            store.dispatch_show_gvk(gvk);
        }
        ToUiSignal::CtrlSPressed => {
            store.dispatch_ctrl_s();
        }
        ToUiSignal::ExecuteCurrent => {
            store.dispatch_shell_current();
        }
        ToUiSignal::CtrlYPressed => {
            store.dispatch_ctrl_y();
        }
        ToUiSignal::F5Pressed => {
            store.dispatch_f5();
        }
        ToUiSignal::EscPressed => {
            store.dispatch_esc();
        }
        ToUiSignal::ShowDebugLog => {
            store.dispatch_show_debug_log();
        }
    }
}

pub fn enter_command_handler_loop(
    ui: &mut CursiveRunnable,
    store: Arc<Mutex<UiStore>>,
) -> anyhow::Result<()> {
    loop {
        ui.try_run_with(init_cursive_backend)?;

        let mut store = store.lock_unwrap();
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
