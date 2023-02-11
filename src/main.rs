use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use cursive::direction::Orientation;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::Info;
use cursive::reexports::log::{error, warn};
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive::{event, menu, CursiveRunnable};
use cursive_table_view::{TableView, TableViewItem};
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use kube::api::GroupVersionKind;
use kube::Client;

use crate::client::{discover_gvk, ReflectorRegistry, ResourceView};
use crate::ui::resource_column::ResourceColumn;
use crate::util::k8s::{GvkExt, GvkStaticExt};
use crate::util::ui::group_gvks;

pub mod client;
pub mod theme;
pub mod ui;
pub mod util;

impl TableViewItem<ResourceColumn> for ResourceView {
    fn to_column(&self, column: ResourceColumn) -> String {
        match column {
            ResourceColumn::Namespace => self.namespace(),
            ResourceColumn::Name => self.name(),
            _ => String::new(),
        }
    }

    fn cmp(&self, other: &Self, column: ResourceColumn) -> Ordering
    where
        Self: Sized,
    {
        match column {
            ResourceColumn::Name => self.name().cmp(&other.name()),
            ResourceColumn::Namespace => self.namespace().cmp(&other.namespace()),
            _ => Ordering::Equal,
        }
    }
}

fn build_menu(
    menu_bar: &mut Menubar,
    gvks: Vec<GroupVersionKind>,
    current_gvk: &Arc<Mutex<GroupVersionKind>>,
) {
    menu_bar.add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    let grouped_gvks = group_gvks(gvks);

    for (group_name, group) in grouped_gvks {
        let mut group_tree = menu::Tree::new();
        for gvk in group {
            let leaf_name = if group_name == "Misc" {
                format!("{}/{}/{}", &gvk.group, &gvk.version, &gvk.kind)
            } else {
                format!("{}/{}", &gvk.version, &gvk.kind)
            };
            let c = Arc::clone(current_gvk);
            group_tree = group_tree.leaf(leaf_name, move |s| {
                if let Ok(mut g) = c.lock() {
                    *g = gvk.clone();
                    s.pop_layer();
                }
            });
        }
        menu_bar.add_subtree(group_name, group_tree);
    }
}

fn build_main_layout(current_gvk: &Arc<Mutex<GroupVersionKind>>) -> LinearLayout {
    let gvk = Arc::clone(current_gvk);
    let _gvk = match gvk.lock() {
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

    let table: TableView<ResourceView, ResourceColumn> = TableView::new()
        .column(
            ResourceColumn::Namespace,
            ResourceColumn::Namespace.as_ref(),
            |c| c,
        )
        .column(ResourceColumn::Name, "Name", |c| c)
        .column(ResourceColumn::Name, "Rate", |c| c);

    let table_panel = Panel::new(table.with_name("table").full_screen()).title("Pods");

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(DummyView {}.full_width());
    main_layout.add_child(table_panel);

    main_layout
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

    let current_gvk = Arc::new(Mutex::new(Pod::gvk_for_type()));

    build_menu(ui.menubar(), discovered_gvks, &current_gvk);
    ui.set_autohide_menu(false);

    cursive::logger::init();
    log::set_max_level(Info);

    let main_layout = build_main_layout(&current_gvk);
    ui.add_fullscreen_layer(main_layout);

    let sink = ui.cb_sink().clone();

    let (sender, receiver) = kanal::unbounded_async();

    let registry = main_rt.block_on(async {
        let mut reg = ReflectorRegistry::new(sender, &client);
        reg.register::<Pod>().await;
        reg.register::<Namespace>().await;
        reg
    });

    let arc_registry = Arc::new(Mutex::new(registry));

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
                let call_result = siv.call_on_name(
                    "table",
                    |table: &mut TableView<ResourceView, ResourceColumn>| {
                        table.take_items();
                        match registry.lock() {
                            Ok(guard) => {
                                if let Some(resources) = guard.get_resources(&resource_gvk) {
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
                );
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
