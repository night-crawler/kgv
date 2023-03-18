use std::sync::Arc;

use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, TextView};

use crate::reexports::Mutex;
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::ViewMeta;
use crate::util::view_with_data::ViewWithMeta;

pub fn build_code_view(
    store: Arc<Mutex<UiStore>>,
    title: String,
    styled_string: StyledString,
) -> ViewWithMeta<ViewMeta> {
    let view_meta = ViewMeta::Dialog {
        id: store.inc_counter(),
        name: format!("Code View {title}"),
    };
    let tv = TextView::new(styled_string)
        .with_name(view_meta.get_unique_name())
        .full_screen()
        .scrollable();
    let dialog = Dialog::around(tv).title(title);
    ViewWithMeta::new(dialog, view_meta)
}
