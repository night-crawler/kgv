use std::sync::Arc;

use cursive::menu;
use cursive::views::Menubar;
use kube::api::GroupVersionKind;
use crate::reexports::Mutex;

use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::UiStore;
use crate::util::ui::group_gvks;

pub fn build_menu(discovered_gvks: Vec<GroupVersionKind>, store: Arc<Mutex<UiStore>>) -> Menubar {
    let (to_backend_sender, to_ui_sender) = {
        let store = store.lock_unwrap();
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
                let r = resource_gvk.clone();
                let r2 = resource_gvk.clone();
                let to_backend_sender = to_backend_sender.clone();

                let chain = ToUiSignal::new_chain()
                    .chain(|_| Some(ToUiSignal::ShowGvk(r)))
                    .chain(move |_| {
                        to_backend_sender.send_unwrap(ToBackendSignal::RequestGvkItems(r2));
                        None
                    });

                to_ui_sender.send_unwrap(chain);
            });
        }
        menubar.add_subtree(group_name, group_tree);
    }

    menubar
}
