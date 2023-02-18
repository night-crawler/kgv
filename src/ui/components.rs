use std::sync::{Arc, Mutex};

use cursive::direction::Orientation;
use cursive::menu;
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;

use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::ui::group_gvks;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::traits::MenuNameExt;
use crate::ui::ui_store::UiStore;

pub fn build_main_layout(store: Arc<Mutex<UiStore>>) -> LinearLayout {
    let (ns_filter, name_filter, columns, to_ui_sender, to_backend_sender, selected_gvk) = {
        let store = store.lock().unwrap();
        (
            store.ns_filter.clone(),
            store.name_filter.clone(),
            store.get_columns(),
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
                    .unwrap();
            })
    };

    let name_edit_view = {
        let to_ui_sender = to_ui_sender.clone();
        EditView::new()
            .content(name_filter)
            .on_submit(move |_, text| {
                to_ui_sender
                    .send(ToUiSignal::ApplyNameFilter(text.into()))
                    .unwrap();
            })
    };

    filter_layout.add_child(
        Panel::new(namespace_edit_view)
            .title("Namespaces")
            .full_width(),
    );
    filter_layout.add_child(Panel::new(name_edit_view).title("Name").full_width());

    let mut table: TableView<ResourceView, ResourceColumn> =
        TableView::new().on_submit(move |siv, _, index| {
            siv.call_on_name(
                "table",
                |table: &mut TableView<ResourceView, ResourceColumn>| {
                    if let Some(resource) = table.borrow_item(index) {
                        to_backend_sender
                            .send(ToBackendSignal::RequestDetails(resource.clone()))
                            .unwrap();
                        to_ui_sender
                            .send(ToUiSignal::ShowDetails(resource.clone()))
                            .unwrap();
                    }
                },
            );
        });

    for column in columns {
        table = table.column(column, column.as_ref(), |mut c| {
            match column {
                ResourceColumn::Namespace => c = c.width(20),
                ResourceColumn::Name => c = c.width_percent(35),
                ResourceColumn::Restarts => c = c.width(7),
                ResourceColumn::Ready => c = c.width(7),
                ResourceColumn::Age => c = c.width(7),
                ResourceColumn::Status => c = c.width(7),
                _ => {}
            }
            c
        });
    }

    let table_panel = Panel::new(table.with_name("table").full_screen())
        .title(selected_gvk.short_menu_name())
        .with_name("panel");

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(DummyView {}.full_width());
    main_layout.add_child(table_panel);

    main_layout
}

pub fn build_menu(discovered_gvks: Vec<GroupVersionKind>, store: Arc<Mutex<UiStore>>) -> Menubar {
    let sender = store.lock().unwrap().to_backend_sender.clone();

    let mut menubar = Menubar::new();
    menubar.add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    let grouped_gvks = group_gvks(discovered_gvks);

    for (group_name, group) in grouped_gvks {
        let mut group_tree = menu::Tree::new();
        for resource_gvk in group {
            let leaf_name = if group_name == "Misc" {
                resource_gvk.full_menu_name()
            } else {
                resource_gvk.short_menu_name()
            };

            let sender = sender.clone();
            group_tree = group_tree.leaf(leaf_name, move |_| {
                sender
                    .send(ToBackendSignal::RequestGvkItems(resource_gvk.clone()))
                    .unwrap();
            });
        }
        menubar.add_subtree(group_name, group_tree);
    }

    menubar
}
