use std::sync::Arc;

use cursive::traits::*;
use cursive::views::Dialog;
use cursive_cached_text_view::CachedTextView;

use crate::model::resource::resource_view::ResourceView;
use crate::reexports::sync::Mutex;
use crate::traits::ext::gvk::GvkExt;
use crate::traits::ext::mutex::MutexExt;
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::ViewMeta;
use crate::util::view_with_data::ViewWithMeta;

pub fn build_code_view(
    store: Arc<Mutex<UiStore>>,
    resource: ResourceView,
) -> anyhow::Result<ViewWithMeta<ViewMeta>> {
    let styled_string = store.lock_sync()?.highlight(&resource)?;

    let view_meta = ViewMeta::Code {
        id: store.inc_counter(),
        gvk: resource.gvk(),
        title: resource.name(),
        uid: resource.uid_or_name(),
    };

    let tv = CachedTextView::new(styled_string, 5)
        .with_name(view_meta.get_unique_name())
        .full_screen()
        .scrollable();
    let dialog = Dialog::around(tv).title(resource.full_unique_name());

    Ok(ViewWithMeta::new(dialog, view_meta))
}
