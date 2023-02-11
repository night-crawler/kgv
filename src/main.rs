use std::ops::Deref;
use std::sync::{Arc, LockResult, Mutex};

use anyhow::Result;
use cursive::direction::Orientation;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::Info;
use cursive::reexports::log::{error, info, warn};
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive::{event, menu, Cursive, CursiveRunnable};
use cursive_table_view::TableView;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use kube::api::GroupVersionKind;
use kube::Client;

use crate::model::discover_gvk;
use crate::model::reflector_registry::ReflectorRegistry;
use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::model::traits::{GvkExt, GvkStaticExt};
use crate::ui::traits::MenuNameExt;
use crate::ui::{group_gvks, GVK_TO_COLUMNS_MAP};

pub mod model;
pub mod theme;
pub mod ui;
pub mod util;

fn build_menu(
    menu_bar: &mut Menubar,
    gvks: Vec<GroupVersionKind>,
    selected_gvk: &Arc<Mutex<GroupVersionKind>>,
) {
    menu_bar.add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    let grouped_gvks = group_gvks(gvks);

    for (group_name, group) in grouped_gvks {
        let mut group_tree = menu::Tree::new();
        for resource_gvk in group {
            let leaf_name = if group_name == "Misc" {
                resource_gvk.full_menu_name()
            } else {
                resource_gvk.short_menu_name()
            };

            let selected_gvk_cloned = Arc::clone(selected_gvk);
            group_tree = group_tree.leaf(leaf_name, move |siv| {
                if let Ok(mut guard) = selected_gvk_cloned.lock() {
                    *guard = resource_gvk.clone();
                }
            });
        }
        menu_bar.add_subtree(group_name, group_tree);
    }
}

fn build_main_layout(selected_gvk: &Arc<Mutex<GroupVersionKind>>) -> LinearLayout {
    let selected_gvk = Arc::clone(selected_gvk);
    let selected_gvk = match selected_gvk.lock() {
        Ok(gvk) => gvk.clone(),
        Err(err) => {
            error!("Failed to get GVK while building menu: {}", err);
            Pod::gvk_for_type()
        }
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);

    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);
    filter_layout.add_child(Panel::new(EditView::new()).title("Namespaces").full_width());
    filter_layout.add_child(Panel::new(EditView::new()).title("Name").full_width());

    let mut table: TableView<ResourceView, ResourceColumn> = TableView::new();

    for column in GVK_TO_COLUMNS_MAP.get(&selected_gvk).into_iter().flatten() {
        table = table.column(*column, column.as_ref(), |c| c);
    }

    let table_panel = Panel::new(table.with_name("table").full_screen()).title("Pods");

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(DummyView {}.full_width());
    main_layout.add_child(table_panel);

    main_layout
}

fn render_all_items(
    siv: &mut Cursive,
    registry: &Arc<Mutex<ReflectorRegistry>>,
    resource_gvk: GroupVersionKind,
) -> Option<()> {
    let registry = Arc::clone(registry);
    siv.call_on_name(
        "table",
        |table: &mut TableView<ResourceView, ResourceColumn>| {
            table.take_items();
            match registry.lock() {
                Ok(guard) => {
                    if let Some(resources) = guard.get_resources(&resource_gvk) {
                        info!("Set items for type {:?}", resource_gvk);
                        table.set_items(resources);
                    } else {
                        error!("GVK {:?} not found in registry", resource_gvk)
                    }
                }
                Err(err) => {
                    error!("Could not acquire a lock {}", err)
                }
            }
        },
    )
}

fn main() -> Result<()> {
    let main_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;

    let exchange_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()?;

    let client = main_rt.block_on(async { Client::try_default().await })?;

    let discovered_gvks = main_rt.block_on(async { discover_gvk(&client).await })?;

    let mut ui = CursiveRunnable::default();
    let mut ui = ui.runner();

    let (sender, receiver) = kanal::unbounded_async();

    let current_gvk = Arc::new(Mutex::new(Pod::gvk_for_type()));
    let registry = main_rt.block_on(async {
        let mut reg = ReflectorRegistry::new(sender, &client);
        reg.register::<Pod>().await;
        reg.register::<Namespace>().await;
        reg
    });
    let arc_registry = Arc::new(Mutex::new(registry));

    build_menu(ui.menubar(), discovered_gvks, &current_gvk);
    ui.set_autohide_menu(false);

    cursive::logger::init();
    log::set_max_level(Info);

    let main_layout = build_main_layout(&current_gvk);
    ui.add_fullscreen_layer(main_layout);

    let sink = ui.cb_sink().clone();

    exchange_rt.spawn(async move {
        let mut stream = receiver.stream();

        while let Some(resource_view) = stream.next().await {
            let resource_gvk = resource_view.gvk();
            if let Ok(guard) = current_gvk.lock() {
                if &resource_gvk != guard.deref() {
                    continue;
                }
            }

            let registry = Arc::clone(&arc_registry);
            let send_result = sink.send(Box::new(move |siv| {
                let call_result = render_all_items(siv, &registry, resource_gvk.clone());

                // let call_result = siv.call_on_name(
                //     "table",
                //     |table: &mut TableView<ResourceView, ResourceColumn>| {
                //         table.take_items();
                //         match registry.lock() {
                //             Ok(guard) => {
                //                 if let Some(resources) = guard.get_resources(&resource_gvk) {
                //                     info!("Set items for type {:?}", resource_gvk);
                //                     table.set_items(resources);
                //                 } else {
                //                     error!("GVK {:?} not found in registry", resource_gvk)
                //                 }
                //             }
                //             Err(err) => {
                //                 error!("Could not acquire a lock {}", err)
                //             }
                //         }
                //     },
                // );
                if call_result.is_none() {
                    warn!("Failed to call a callback for {:?}", resource_gvk);
                }
            }));

            if let Err(err) = send_result {
                error!(
                    "Failed to send an update event to table for resource {:?}; Error: {}",
                    resource_view.gvk(),
                    err
                );
            }
        }
        warn!("Main exchange loop has ended")
    });

    ui.add_global_callback('~', |s| s.toggle_debug_console());
    ui.add_global_callback(event::Key::Esc, |s| s.select_menubar());

    ui.run();

    Ok(())
}
