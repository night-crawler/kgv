use std::sync::{Arc, Mutex};

use crate::model::pod::pod_container_column::PodContainerColumn;
use crate::model::pod::pod_container_view::PodContainerView;
use crate::model::resource::resource_column::ResourceColumn;
use crate::model::resource::resource_view::ResourceView;
use cursive::direction::Orientation;
use cursive::menu;
use cursive::theme::BaseColor;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive_table_view::TableView;
use k8s_openapi::api::core::v1::Pod;
use kube::api::GroupVersionKind;
use strum::IntoEnumIterator;

use crate::model::traits::GvkExt;
use crate::ui::group_gvks;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::traits::MenuNameExt;
use crate::ui::ui_store::UiStore;
use crate::util::panics::ResultExt;

pub fn build_pod_detail_layout(store: Arc<Mutex<UiStore>>) -> LinearLayout {
    let mut main_layout = LinearLayout::vertical();

    let panel_title = {
        let store = store.lock().unwrap_or_log();
        if let Some(resource) = store.selected_resource.as_ref() {
            let mut styled =
                StyledString::styled(resource.gvk().full_menu_name(), BaseColor::Blue.light());

            styled.append(StyledString::styled(
                format!(" [{}/{}]", resource.namespace(), resource.name()),
                BaseColor::Green.light(),
            ));
            styled
        } else {
            StyledString::new()
        }
    };

    let mut table: TableView<PodContainerView, PodContainerColumn> =
        TableView::new().on_submit(move |siv, _row, _index| {
            let mut store = store.lock().unwrap_or_log();
            store.interactive_command =
                Some(InteractiveCommand::Exec(Pod::default(), "".to_string()));
            siv.quit();
        });

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
    let (ns_filter, name_filter, columns, to_ui_sender, to_backend_sender, selected_gvk) = {
        let store = store.lock().unwrap_or_log();
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

    let mut table: TableView<ResourceView, ResourceColumn> =
        TableView::new().on_submit(move |siv, _, index| {
            siv.call_on_name(
                "table",
                |table: &mut TableView<ResourceView, ResourceColumn>| {
                    if let Some(resource) = table.borrow_item(index) {
                        to_backend_sender
                            .send(ToBackendSignal::RequestDetails(resource.clone()))
                            .unwrap_or_log();
                        to_ui_sender
                            .send(ToUiSignal::ShowDetails(resource.clone()))
                            .unwrap_or_log();
                    }
                },
            );
        });

    for column in columns {
        table = table.column(column, column.as_ref(), |c| match column {
            ResourceColumn::Namespace => c.width(20),
            ResourceColumn::Name => c.width_percent(35),
            ResourceColumn::Restarts => c.width(7),
            ResourceColumn::Ready => c.width(7),
            ResourceColumn::Age => c.width(7),
            ResourceColumn::Status => c.width(7),
            _ => c,
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
                resource_gvk.full_menu_name()
            } else {
                resource_gvk.short_menu_name()
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
