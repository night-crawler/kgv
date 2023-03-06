use std::sync::{Arc, Mutex};

use cursive::menu;
use cursive::views::Menubar;
use kube::api::GroupVersionKind;

use crate::traits::ext::gvk::GvkNameExt;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::UiStore;
use crate::util::panics::ResultExt;
use crate::util::ui::group_gvks;

pub fn build_menu(discovered_gvks: Vec<GroupVersionKind>, store: Arc<Mutex<UiStore>>) -> Menubar {
    let (to_backend_sender, to_ui_sender) = {
        let store = store.lock().unwrap_or_log();
        (store.to_backend_sender.clone(), store.to_ui_sender.clone())
    };

    let mut menubar = Menubar::new();
    menubar.add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    let grouped_gvks = group_gvks(discovered_gvks);

    for (group_name, group) in grouped_gvks {
        let mut group_tree = menu::Tree::new();
        for resource_gvk in group {
            let leaf_name = if group_name == "Misc" {
                resource_gvk.full_name()
            } else {
                resource_gvk.short_name()
            };

            let to_backend_sender = to_backend_sender.clone();
            let to_ui_sender = to_ui_sender.clone();
            group_tree = group_tree.leaf(leaf_name, move |_| {
                to_ui_sender
                    .send(ToUiSignal::ShowGvk(resource_gvk.clone()))
                    .unwrap_or_log();
                to_backend_sender
                    .send(ToBackendSignal::RequestGvkItems(resource_gvk.clone()))
                    .unwrap_or_log();
            });
        }
        menubar.add_subtree(group_name, group_tree);
    }

    menubar
}
