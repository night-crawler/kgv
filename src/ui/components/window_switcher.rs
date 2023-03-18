use std::sync::Arc;

use cursive::traits::Nameable;
use cursive::views::{Dialog, LinearLayout, Panel, SelectView};

use crate::reexports::{Mutex, RwLock};
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::ui::signals::ToUiSignal;
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::ViewMeta;
use crate::util::ui::build_edit_view;
use crate::util::view_with_data::ViewWithMeta;

pub fn build_window_switcher(store: Arc<Mutex<UiStore>>) -> ViewWithMeta<ViewMeta> {
    let (view_meta_list, counter, to_ui_sender) = {
        let mut store = store.lock_unwrap();
        (
            store.get_filtered_windows(""),
            store.inc_counter(),
            store.to_ui_sender.clone(),
        )
    };

    let meta = ViewMeta::WindowSwitcher { id: counter };
    let list_name = meta.get_unique_name();
    let view_edit_name = meta.get_edit_name("windows");

    let mut layout = LinearLayout::vertical();
    let store = Arc::clone(&store);
    let edit = build_edit_view(view_edit_name, "", move |siv, text, _| {
        siv.call_on_name(
            &list_name,
            |view: &mut SelectView<Arc<RwLock<ViewMeta>>>| {
                view.clear();
                let store = store.lock_unwrap();
                store
                    .get_filtered_windows(text)
                    .into_iter()
                    .for_each(|(title, view_meta)| {
                        view.add_item(title, view_meta);
                    });
            },
        );
    });
    let list_name = meta.get_unique_name();

    let mut select_view: SelectView<Arc<RwLock<ViewMeta>>> = SelectView::new();
    let meta = Arc::new(RwLock::new(meta));

    view_meta_list.into_iter().for_each(|(title, view_meta)| {
        select_view.add_item(title, view_meta);
    });

    select_view.set_on_submit(move |_, item: &Arc<RwLock<ViewMeta>>| {
        let id = item.read_unwrap().get_id();
        to_ui_sender.send_unwrap(ToUiSignal::ShowWindow(id));
    });

    layout.add_child(edit);
    layout.add_child(Panel::new(select_view.with_name(list_name)).title("Windows"));

    let dialog = Dialog::around(layout);
    ViewWithMeta {
        inner: Box::new(dialog),
        meta,
    }
}
