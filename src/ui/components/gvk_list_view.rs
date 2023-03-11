use std::sync::{Arc, Mutex, RwLock};

use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::views::{DummyView, LinearLayout, NamedView, Panel};
use cursive_table_view::TableView;

use crate::model::resource::resource_view::EvaluatedResource;
use crate::traits::ext::gvk::GvkNameExt;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::{Filter, ViewMeta};
use crate::util::panics::ResultExt;
use crate::util::ui::build_edit_view;
use crate::util::view_with_data::ViewWithData;

trait UiStoreComponentExt {
    fn build_list_view_table(
        &self,
        view_meta: &ViewMeta,
    ) -> NamedView<TableView<EvaluatedResource, usize>>;
}

impl UiStoreComponentExt for Arc<Mutex<UiStore>> {
    fn build_list_view_table(
        &self,
        view_meta: &ViewMeta,
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
            let name = view_meta.get_unique_name();
            table.set_on_submit(move |siv, _, index| {
                siv.call_on_name(&name, |table: &mut TableView<EvaluatedResource, usize>| {
                    store.lock().unwrap_or_log().selected_resource =
                        table.borrow_item(index).cloned();
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
            let name = view_meta.get_unique_name();
            table.set_on_select(move |siv, _, index| {
                siv.call_on_name(&name, |table: &mut TableView<EvaluatedResource, usize>| {
                    store.lock().unwrap_or_log().selected_resource =
                        table.borrow_item(index).cloned();
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

        table.with_name(view_meta.get_unique_name())
    }
}

pub fn build_gvk_list_view_layout(store: Arc<Mutex<UiStore>>) -> ViewWithData<ViewMeta> {
    let (to_ui_sender, selected_gvk, counter) = {
        let mut store = store.lock().unwrap_or_log();
        store.counter += 1;
        (
            store.to_ui_sender.clone(),
            store.selected_gvk.clone(),
            store.counter,
        )
    };

    let view_meta = ViewMeta::List {
        id: counter,
        gvk: selected_gvk.clone(),
        filter: Filter::default(),
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);
    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);

    let namespace_edit_view = {
        let sender = to_ui_sender.clone();
        build_edit_view(
            view_meta.get_edit_name("namespace"),
            "".to_string(),
            move |_, text, _| {
                sender
                    .send(ToUiSignal::ApplyNamespaceFilter(counter, text.into()))
                    .unwrap_or_log();
            },
        )
    };

    let name_edit_view = {
        build_edit_view(
            view_meta.get_edit_name("name"),
            "".to_string(),
            move |_, text, _| {
                to_ui_sender
                    .send(ToUiSignal::ApplyNameFilter(counter, text.into()))
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

    let table = store.build_list_view_table(&view_meta);

    let table_panel = Panel::new(table.full_screen())
        .title(selected_gvk.short_name())
        .with_name(view_meta.get_panel_name());

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(DummyView {}.full_width());
    main_layout.add_child(table_panel);

    ViewWithData {
        inner: Box::new(main_layout),
        data: Arc::new(RwLock::new(view_meta)),
    }
}
