use cursive::reexports::log::info;
use cursive::traits::*;
use cursive::views::{LinearLayout, Panel};

use crate::model::resource::resource_view::ResourceView;
use crate::traits::ext::gvk::{GvkExt, GvkNameExt};

pub fn build_detail_view(resource: ResourceView, html: String) -> LinearLayout {
    let mut main_layout = LinearLayout::vertical();

    let mut view = cursive_markup::MarkupView::html(&html);
    view.on_link_focus(|_, url| {
        info!("Focused a link: {url}");
    });
    view.on_link_select(|_, url| {
        info!("Selected a link: {url}");
    });

    let title = format!("{} {}", resource.gvk().full_name(), resource.name());

    let panel = Panel::new(view.scrollable()).title(title);

    main_layout.add_child(panel.with_name("details").full_screen());

    main_layout
}
