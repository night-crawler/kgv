use std::sync::{Arc, Mutex};
use std::time::Duration;

use cursive::reexports::crossbeam_channel::internal::SelectHandle;
use cursive::reexports::log::warn;

use crate::ui::signals::ToUiSignal;
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::util::panics::ResultExt;

pub fn dispatch_events(
    store: Arc<Mutex<UiStore>>,
    from_backend_receiver: kanal::Receiver<ToUiSignal>,
) {
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
                    ToUiSignal::CtrlSPressed => {
                        store.dispatch_ctrl_s();
                    }
                    ToUiSignal::ExecuteCurrent => {
                        store.dispatch_shell_current();
                    }
                    ToUiSignal::CtrlYPressed => {
                        store.dispatch_ctrl_y();
                    }
                }
            }
        })
        .unwrap_or_log();
}
