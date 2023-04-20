use std::sync::Arc;

use cursive::menu;
use cursive::views::Menubar;
use kube::api::GroupVersionKind;

use crate::reexports::sync::Mutex;
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::ui::components::gvk_switcher::build_gvk_show_chain;
use crate::ui::ui_store::UiStore;
use crate::util::ui::group_gvks;

pub(crate) fn build_menu(
    discovered_gvks: Vec<GroupVersionKind>,
    store: Arc<Mutex<UiStore>>,
) -> Menubar {
    let (to_backend_sender, to_ui_sender) = {
        let store = store.lock_unwrap();
        (
            store.to_backend_sender.clone(),
            store.inter_ui_sender.clone(),
        )
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
                let to_backend_sender = to_backend_sender.clone();
                let chain = build_gvk_show_chain(to_backend_sender, &resource_gvk);

                to_ui_sender.send_unwrap(chain);
            });
        }
        menubar.add_subtree(group_name, group_tree);
    }

    menubar
}
