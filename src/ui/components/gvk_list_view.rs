use std::cmp::Ordering;
use std::sync::Arc;

use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::views::{LinearLayout, Panel};
use cursive_table_view::TableView;
use kube::core::GroupVersionKind;

use crate::model::resource::resource_view::EvaluatedResource;
use crate::reexports::sync::Mutex;
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::traits::ext::table_view::TableCallBacks;
use crate::ui::signals::InterUiSignal;
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::{ListViewFilter, ViewMeta};
use crate::util::ui::build_edit_view;
use crate::util::view_with_data::ViewWithMeta;

trait UiStoreComponentExt {
    fn build_list_view_table(&self, gvk: &GroupVersionKind) -> TableView<EvaluatedResource, usize>;
}

impl UiStoreComponentExt for Arc<Mutex<UiStore>> {
    fn build_list_view_table(&self, gvk: &GroupVersionKind) -> TableView<EvaluatedResource, usize> {
        let column_handles = {
            self.lock_unwrap()
                .resource_manager
                .read_unwrap()
                .get_columns(gvk)
        };

        let mut table: TableView<EvaluatedResource, usize> = TableView::new();

        for (index, column) in column_handles.iter().enumerate() {
            table = table.column(
                index,
                &column.display_name,
                |c| {
                    if column.width != 0 {
                        c.width(column.width)
                    } else {
                        c
                    }
                    .ordering(Ordering::Less)
                },
                true,
            );
        }

        table
    }
}

pub(crate) fn build_gvk_list_view_layout(store: Arc<Mutex<UiStore>>) -> ViewWithMeta<ViewMeta> {
    let (to_ui_sender, selected_gvk, counter) = {
        let mut store = store.lock_unwrap();
        (
            store.inter_ui_sender.clone(),
            store.selected_gvk.clone(),
            store.inc_counter(),
        )
    };

    let view_meta = ViewMeta::List {
        id: counter,
        gvk: selected_gvk.clone(),
        filter: ListViewFilter::default(),
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);
    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);

    let namespace_edit_view = {
        let sender = to_ui_sender.clone();
        build_edit_view(
            view_meta.get_edit_name("namespace"),
            "".to_string(),
            move |_, text, _| {
                sender.send_unwrap(InterUiSignal::ApplyNamespaceFilter(counter, text.into()));
            },
        )
    };

    let name_edit_view = {
        let to_ui_sender = to_ui_sender.clone();
        build_edit_view(
            view_meta.get_edit_name("name"),
            "".to_string(),
            move |_, text, _| {
                to_ui_sender.send_unwrap(InterUiSignal::ApplyNameFilter(counter, text.into()))
            },
        )
    };

    filter_layout.add_child(Panel::new(name_edit_view).title("Name").full_width());
    filter_layout.add_child(
        Panel::new(namespace_edit_view)
            .title("Namespaces")
            .full_width(),
    );

    let mut table = store.build_list_view_table(&selected_gvk);
    {
        table.set_on_submit_named(
            &view_meta.get_unique_name(),
            move |_, evaluated_resource| {
                to_ui_sender.send_unwrap(InterUiSignal::ShowDetails(evaluated_resource.resource));
            },
        );
    }

    let table = table.with_name(view_meta.get_unique_name());

    let table_panel = Panel::new(table.full_screen())
        .title(selected_gvk.short_name())
        .with_name(view_meta.get_panel_name());

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(table_panel);

    ViewWithMeta::new(main_layout, view_meta)
}
