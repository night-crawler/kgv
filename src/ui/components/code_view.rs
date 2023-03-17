use std::sync::Arc;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, TextView};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::ViewMeta;
use crate::util::view_with_data::ViewWithMeta;
use crate::reexports::Mutex;

pub fn build_code_view(store: Arc<Mutex<UiStore>>, name: String, styled_string: StyledString) -> ViewWithMeta<ViewMeta> {
    let view_meta = ViewMeta::Dialog {
        id: store.inc_counter(),
        name: "Code View".to_string(),
    };

    let tv = TextView::new(styled_string).full_screen().scrollable();
    let dialog = Dialog::around(tv).title(name);
    ViewWithMeta::new(dialog, view_meta)
}
