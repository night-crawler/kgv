use std::sync::Arc;

use cursive::traits::{Nameable, Resizable, Scrollable};
use cursive::views::{Dialog, LinearLayout, Panel, SelectView};
use kube::core::GroupVersionKind;

use crate::reexports::sync::{Mutex, RwLock};
use crate::traits::ext::cursive::SivUtilExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::ViewMeta;
use crate::util::ui::build_edit_view;
use crate::util::view_with_data::ViewWithMeta;

pub fn build_gvk_switcher(store: Arc<Mutex<UiStore>>) -> ViewWithMeta<ViewMeta> {
    let (to_ui_sender, to_backend_sender, gvks, counter) = {
        let mut store = store.lock_unwrap();
        (
            store.to_ui_sender.clone(),
            store.to_backend_sender.clone(),
            store.get_filtered_gvks(""),
            store.inc_counter(),
        )
    };

    let meta = ViewMeta::GvkSwitcher { id: counter };
    let list_name = meta.get_unique_name();
    let view_edit_name = meta.get_edit_name("gvks");

    let mut layout = LinearLayout::vertical();
    let store = Arc::clone(&store);

    let selected = Arc::new(Mutex::new(gvks.first().cloned().map(|(_, gvk)| gvk)));

    let mut select_view: SelectView<GroupVersionKind> = SelectView::new();
    gvks.into_iter().for_each(|(title, gvk)| {
        select_view.add_item(title, gvk);
    });

    let mut edit = {
        let store = Arc::clone(&store);
        let list_name = list_name.clone();
        let selected = Arc::clone(&selected);
        build_edit_view(view_edit_name, "", move |siv, text, _| {
            siv.call_on_name(&list_name, |view: &mut SelectView<GroupVersionKind>| {
                let store = store.lock_unwrap();
                let filtered = store.get_filtered_gvks(text);
                *selected.lock_unwrap() = filtered.first().cloned().map(|(_, gvk)| gvk);

                view.clear();
                for (name, gvk) in filtered {
                    view.add_item(name, gvk);
                }
            });
        })
    };
    let meta = Arc::new(RwLock::new(meta));

    {
        // edit on submit
        let view_meta = Arc::clone(&meta);
        let selected = Arc::clone(&selected);
        let to_backend_sender = to_backend_sender.clone();
        let to_ui_sender = to_ui_sender.clone();
        let store = Arc::clone(&store);

        edit.get_mut().set_on_submit(move |siv, _| {
            let to_backend_sender = to_backend_sender.clone();
            siv.remove_views(vec![Arc::clone(&view_meta)]);
            store.lock_unwrap().view_stack.remove_gvk_switchers();

            if let Some(gvk) = selected.lock_unwrap().as_ref() {
                to_ui_sender.send_unwrap(build_gvk_show_chain(to_backend_sender, gvk));
            }
        });
    }

    {
        // select view on submit
        let store = Arc::clone(&store);
        let view_meta = Arc::clone(&meta);
        select_view.set_on_submit(move |siv, gvk: &GroupVersionKind| {
            siv.remove_views(vec![Arc::clone(&view_meta)]);
            store.lock_unwrap().view_stack.remove_gvk_switchers();

            let to_backend_sender = to_backend_sender.clone();
            to_ui_sender.send_unwrap(build_gvk_show_chain(to_backend_sender, gvk));
        });
    }

    {
        // select view on select
        let selected = Arc::clone(&selected);
        select_view.set_on_select(move |_, item| {
            let _ = selected.lock_unwrap().insert(item.clone());
        });
    }

    layout.add_child(edit);
    layout.add_child(Panel::new(select_view.with_name(list_name).scrollable()));

    let dialog = Dialog::around(layout).title("Registered GVKs");
    ViewWithMeta {
        inner: Box::new(dialog.full_height()),
        meta,
    }
}

pub fn build_gvk_show_chain(
    to_backend_sender: kanal::Sender<ToBackendSignal>,
    gvk: &GroupVersionKind,
) -> ToUiSignal {
    let (gvk1, gvk2) = (gvk.clone(), gvk.clone());
    ToUiSignal::new_chain()
        .chain(|_| Some(ToUiSignal::ShowGvk(gvk1)))
        .chain(move |_| {
            to_backend_sender.send_unwrap(ToBackendSignal::RequestGvkItems(gvk2));
            None
        })
}
