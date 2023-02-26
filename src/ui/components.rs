use std::sync::{Arc, Mutex};

use cursive::direction::Orientation;
use cursive::menu;
use cursive::theme::BaseColor;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::{Dialog, DummyView, EditView, LinearLayout, Menubar, Panel, TextView};
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;
use strum::IntoEnumIterator;

use crate::model::pod::pod_container_column::PodContainerColumn;
use crate::model::pod::pod_container_view::PodContainerView;
use crate::model::resource::resource_view::EvaluatedResource;
use crate::traits::ext::gvk::{GvkExt, GvkNameExt};
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::util::panics::ResultExt;
use crate::util::ui::group_gvks;

pub fn build_code_view(styled_string: StyledString) -> Dialog {
    let tv = TextView::new(styled_string).full_screen().scrollable();
    Dialog::around(tv)
}

pub fn build_pod_detail_layout(store: Arc<Mutex<UiStore>>) -> LinearLayout {
    let mut main_layout = LinearLayout::vertical();

    let panel_title = {
        let store = store.lock().unwrap_or_log();
        if let Some(evaluated_resource) = store.selected_resource.as_ref() {
            let mut styled = StyledString::styled(
                evaluated_resource.resource.gvk().full_name(),
                BaseColor::Blue.light(),
            );

            styled.append(StyledString::styled(
                format!(
                    " [{}/{}]",
                    evaluated_resource.resource.namespace(),
                    evaluated_resource.resource.name()
                ),
                BaseColor::Green.light(),
            ));
            styled
        } else {
            StyledString::new()
        }
    };

    let mut table: TableView<PodContainerView, PodContainerColumn> = TableView::new();
    {
        let store = store.clone();
        table.set_on_submit(move |_siv, _row, index| {
            store.set_selected_container_by_index(index);
            let to_ui_sender = store.lock().unwrap_or_log().to_ui_sender.clone();
            to_ui_sender
                .send(ToUiSignal::ExecuteCurrent)
                .unwrap_or_log();
        });
    }

    {
        table.set_on_select(move |_siv, _row, index| {
            store.set_selected_container_by_index(index);
        });
    }

    for column in PodContainerColumn::iter() {
        table = table.column(column, column.to_string(), |c| match column {
            PodContainerColumn::Name => c.width(10),
            PodContainerColumn::Ready => c.width(4),
            PodContainerColumn::State => c.width(5),
            PodContainerColumn::Init => c.width(4),
            PodContainerColumn::Restarts => c.width(4),
            PodContainerColumn::Probes => c.width(4),
            PodContainerColumn::Cpu => c.width(8),
            PodContainerColumn::Mem => c.width(8),
            PodContainerColumn::Age => c.width(3),
            _ => c,
        });
    }

    let panel = Panel::new(table.with_name("containers").full_screen()).title(panel_title);
    main_layout.add_child(panel);

    main_layout
}

pub fn build_main_layout(store: Arc<Mutex<UiStore>>) -> LinearLayout {
    let (ns_filter, name_filter, column_handles, to_ui_sender, to_backend_sender, selected_gvk) = {
        let store = store.lock().unwrap_or_log();
        (
            store.ns_filter.clone(),
            store.name_filter.clone(),
            store
                .resource_manager
                .get_column_handles(&store.selected_gvk),
            store.to_ui_sender.clone(),
            store.to_backend_sender.clone(),
            store.selected_gvk.clone(),
        )
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);
    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);

    let namespace_edit_view = {
        let sender = to_ui_sender.clone();
        EditView::new()
            .content(ns_filter)
            .on_submit(move |_, text| {
                sender
                    .send(ToUiSignal::ApplyNamespaceFilter(text.into()))
                    .unwrap_or_log();
            })
    };

    let name_edit_view = {
        let to_ui_sender = to_ui_sender.clone();
        EditView::new()
            .content(name_filter)
            .on_submit(move |_, text| {
                to_ui_sender
                    .send(ToUiSignal::ApplyNameFilter(text.into()))
                    .unwrap_or_log();
            })
    };

    filter_layout.add_child(
        Panel::new(namespace_edit_view)
            .title("Namespaces")
            .full_width(),
    );
    filter_layout.add_child(Panel::new(name_edit_view).title("Name").full_width());

    let mut table: TableView<EvaluatedResource, usize> = TableView::new();

    {
        let store = store.clone();
        table.set_on_submit(move |siv, _, index| {
            siv.call_on_name(
                "table",
                |table: &mut TableView<EvaluatedResource, usize>| {
                    store.set_selected_resource_by_index(index);
                    if let Some(evaluated) = table.borrow_item(index) {
                        to_backend_sender
                            .send(ToBackendSignal::RequestDetails(evaluated.resource.clone()))
                            .unwrap_or_log();
                        to_ui_sender
                            .send(ToUiSignal::ShowDetails(evaluated.resource.clone()))
                            .unwrap_or_log();
                    }
                },
            );
        });
    }

    table.set_on_select(move |siv, _, index| {
        siv.call_on_name(
            "table",
            |_table: &mut TableView<EvaluatedResource, usize>| {
                store.set_selected_resource_by_index(index);
            },
        );
    });

    for (index, column) in column_handles.iter().enumerate() {
        table = table.column(index, &column.display_name, |c| {
            if column.width != 0 {
                c.width(column.width)
            } else {
                c
            }
        });
    }

    let table_panel = Panel::new(table.with_name("table").full_screen())
        .title(selected_gvk.short_name())
        .with_name("panel");

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(DummyView {}.full_width());
    main_layout.add_child(table_panel);

    main_layout
}

pub fn build_menu(discovered_gvks: Vec<GroupVersionKind>, store: Arc<Mutex<UiStore>>) -> Menubar {
    let (to_backend_sender, to_ui_sender) = {
        let store = store.lock().unwrap_or_log();
        (store.to_backend_sender.clone(), store.to_ui_sender.clone())
    };

    let mut menubar = Menubar::new();
    menubar.add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    let grouped_gvks = group_gvks(discovered_gvks);

    for (group_name, group) in grouped_gvks {
        let mut group_tree = menu::Tree::new();
        for resource_gvk in group {
            let leaf_name = if group_name == "Misc" {
                resource_gvk.full_name()
            } else {
                resource_gvk.short_name()
            };

            let to_backend_sender = to_backend_sender.clone();
            let to_ui_sender = to_ui_sender.clone();
            group_tree = group_tree.leaf(leaf_name, move |_| {
                to_ui_sender
                    .send(ToUiSignal::ShowGvk(resource_gvk.clone()))
                    .unwrap_or_log();
                to_backend_sender
                    .send(ToBackendSignal::RequestGvkItems(resource_gvk.clone()))
                    .unwrap_or_log();
            });
        }
        menubar.add_subtree(group_name, group_tree);
    }

    menubar
}
