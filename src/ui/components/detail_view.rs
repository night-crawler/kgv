use std::sync::Arc;

use cursive::reexports::log::info;
use cursive::traits::*;
use cursive::views::{LinearLayout, Panel};

use crate::model::resource::resource_view::ResourceView;
use crate::reexports::sync::Mutex;
use crate::traits::ext::gvk::{GvkExt, GvkNameExt};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::ViewMeta;
use crate::util::view_with_data::ViewWithMeta;

pub(crate) fn build_detail_view(
    store: Arc<Mutex<UiStore>>,
    resource: ResourceView,
    html: String,
) -> ViewWithMeta<ViewMeta> {
    let mut view = cursive_markup::MarkupView::html(&html);
    view.on_link_focus(|_, url| {
        info!("Focused a link: {url}");
    });
    view.on_link_select(|_, url| {
        info!("Selected a link: {url}");
    });

    let meta = ViewMeta::Details {
        id: store.inc_counter(),
        gvk: resource.gvk(),
        name: resource.name(),
        uid: resource.uid_or_name(),
    };

    let title = format!("{} {}", resource.gvk().full_name(), resource.name());

    let mut main_layout = LinearLayout::vertical();
    let view = view.with_name(meta.get_unique_name()).scrollable();
    let panel = Panel::new(view).title(title);
    main_layout.add_child(panel.full_screen());

    ViewWithMeta::new(main_layout, meta)
}
