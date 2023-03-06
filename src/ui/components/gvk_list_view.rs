use std::sync::{Arc, Mutex};

use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::views::{DummyView, LinearLayout, NamedView, Panel};
use cursive_table_view::TableView;
use kube::core::GroupVersionKind;

use crate::model::resource::resource_view::EvaluatedResource;
use crate::traits::ext::gvk::{GvkNameExt, GvkUiExt};
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::util::panics::ResultExt;
use crate::util::ui::build_edit_view;

trait UiStoreComponentExt {
    fn build_list_view_table(
        &self,
        gvk: GroupVersionKind,
    ) -> NamedView<TableView<EvaluatedResource, usize>>;
}

impl UiStoreComponentExt for Arc<Mutex<UiStore>> {
    fn build_list_view_table(
        &self,
        gvk: GroupVersionKind,
    ) -> NamedView<TableView<EvaluatedResource, usize>> {
        let (column_handles, to_ui_sender, to_backend_sender) = {
            let store = self.lock().unwrap_or_log();
            (
                store
                    .resource_manager
                    .get_column_handles(&store.selected_gvk),
                store.to_ui_sender.clone(),
                store.to_backend_sender.clone(),
            )
        };

        let mut table: TableView<EvaluatedResource, usize> = TableView::new();

        {
            let store = Arc::clone(self);
            let name = gvk.list_view_table_name();
            table.set_on_submit(move |siv, _, index| {
                store.set_selected_resource_by_index(&name, index);
                siv.call_on_name(&name, |table: &mut TableView<EvaluatedResource, usize>| {
                    if let Some(evaluated) = table.borrow_item(index) {
                        to_backend_sender
                            .send(ToBackendSignal::RequestDetails(evaluated.resource.clone()))
                            .unwrap_or_log();
                        to_ui_sender
                            .send(ToUiSignal::ShowDetails(evaluated.resource.clone()))
                            .unwrap_or_log();
                    }
                });
            });
        }

        {
            let store = Arc::clone(self);
            let name = gvk.list_view_table_name();
            table.set_on_select(move |siv, _, index| {
                siv.call_on_name(&name, |_table: &mut TableView<EvaluatedResource, usize>| {
                    store.set_selected_resource_by_index(&name, index);
                });
            });
        }

        for (index, column) in column_handles.iter().enumerate() {
            table = table.column(index, &column.display_name, |c| {
                if column.width != 0 {
                    c.width(column.width)
                } else {
                    c
                }
            });
        }

        table.with_name(gvk.list_view_table_name())
    }
}

pub fn build_gvk_list_view_layout(store: Arc<Mutex<UiStore>>) -> LinearLayout {
    let (filter, to_ui_sender, selected_gvk) = {
        let mut store = store.lock().unwrap_or_log();
        let filter = store.get_or_create_filter_for_selected_gvk().clone();
        (
            filter,
            store.to_ui_sender.clone(),
            store.selected_gvk.clone(),
        )
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);
    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);

    let namespace_edit_view = {
        let selected_gvk = selected_gvk.clone();
        let sender = to_ui_sender.clone();
        build_edit_view(
            selected_gvk.namespace_edit_view_name(),
            filter.namespace.to_string(),
            move |_, text, _| {
                sender
                    .send(ToUiSignal::ApplyNamespaceFilter(
                        selected_gvk.clone(),
                        text.into(),
                    ))
                    .unwrap_or_log();
            },
        )
    };

    let name_edit_view = {
        let selected_gvk = selected_gvk.clone();
        build_edit_view(
            selected_gvk.name_edit_view_name(),
            filter.name,
            move |_, text, _| {
                to_ui_sender
                    .send(ToUiSignal::ApplyNameFilter(
                        selected_gvk.clone(),
                        text.into(),
                    ))
                    .unwrap_or_log();
            },
        )
    };

    filter_layout.add_child(
        Panel::new(namespace_edit_view)
            .title("Namespaces")
            .full_width(),
    );
    filter_layout.add_child(Panel::new(name_edit_view).title("Name").full_width());

    let table = store.build_list_view_table(selected_gvk.clone());

    let table_panel = Panel::new(table.full_screen())
        .title(selected_gvk.short_name())
        .with_name(selected_gvk.list_view_panel_name());

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(DummyView {}.full_width());
    main_layout.add_child(table_panel);

    main_layout
}
