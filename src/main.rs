use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use cruet::Inflector;
use cursive::align::HAlign;
use cursive::direction::Orientation;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::{Debug, Info};
use cursive::traits::*;
use cursive::views::{Dialog, DummyView, LinearLayout, Menubar, Panel, ResizedView};
use cursive::{event, menu, Cursive, CursiveRunnable, Printer};
use cursive_table_view::{TableView, TableViewItem};
use futures::StreamExt;
use itertools::Itertools;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use kube::api::GroupVersionKind;
use kube::{Client, Resource, ResourceExt};

use crate::client::{discover, discover_gvk, ReflectorRegistry, ResourceView};

pub mod client;
pub mod theme;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum PodColumn {
    Namespace,
    Name,
    Ready,
    Restarts,
    Status,
    Ip,
    Node,
    Age,
}

impl TableViewItem<PodColumn> for ResourceView {
    fn to_column(&self, column: PodColumn) -> String {
        match column {
            PodColumn::Namespace => self.namespace(),
            PodColumn::Name => self.name(),
            _ => String::new(),
        }
    }

    fn cmp(&self, other: &Self, column: PodColumn) -> Ordering
    where
        Self: Sized,
    {
        match column {
            PodColumn::Name => self.name().cmp(&other.name()),
            PodColumn::Namespace => self.namespace().cmp(&other.namespace()),
            _ => Ordering::Equal,
        }
    }
}

fn gvk_sort_key(gvk: &GroupVersionKind) -> (String, String, String) {
    (gvk.group.clone(), gvk.version.clone(), gvk.kind.clone())
}

fn group_gvks(gvks: Vec<GroupVersionKind>) -> Vec<(String, Vec<GroupVersionKind>)> {
    let mut misc = vec![];
    let mut map = BTreeMap::new();

    for gvk in gvks {
        let grouper = if gvk.kind == "CustomResourceDefinition" {
            "default"
        } else if gvk.kind.starts_with("PersistentVolume") {
            "storage"
        } else if gvk.group.is_empty() {
            "default"
        } else if gvk.group.contains("admission") {
            "admission"
        } else if gvk.group.contains("flow") {
            "flow"
        } else {
            gvk.group.split('.').next().unwrap()
        }
        .to_title_case();
        map.entry(grouper).or_insert_with(Vec::new).push(gvk);
    }

    let default = map.remove("Default");
    let mut grouped = vec![];
    for (grouper, mut group) in map.into_iter() {
        if group.len() == 1 {
            misc.extend(group);
        } else {
            group.sort_unstable_by_key(gvk_sort_key);
            grouped.push((grouper, group));
        }
    }
    misc.sort_unstable_by_key(gvk_sort_key);

    grouped.sort_unstable_by_key(|(name, _)| name.clone());
    if let Some(group) = default {
        grouped.insert(0, ("Default".to_string(), group));
    }

    grouped.push(("Misc".to_string(), misc));
    grouped
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

    let gvks = main_rt.block_on(async { discover_gvk(&client).await })?;
    let grouped_gvks = group_gvks(gvks);

    let mut ui = CursiveRunnable::default();
    let mut ui = ui.runner();

    ui.menubar()
        .add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    for (group_name, group) in grouped_gvks {
        let mut group_tree = menu::Tree::new();
        for gvk in group {
            let leaf_name = if group_name == "Misc" {
                format!("{}/{}/{}", gvk.group, gvk.version, gvk.kind)
            } else {
                format!("{}/{}", gvk.version, gvk.kind)
            };
            group_tree = group_tree.leaf(leaf_name, |s| s.quit());
        }
        ui.menubar().add_subtree(group_name, group_tree);
    }

    ui.set_autohide_menu(false);

    cursive::logger::init();
    log::set_max_level(Debug);

    let mut layout = LinearLayout::new(Orientation::Horizontal);

    let table: TableView<ResourceView, PodColumn> = TableView::new()
        .column(PodColumn::Name, "Name", |c| c.width_percent(20))
        .column(PodColumn::Namespace, "Namespace", |c| {
            c.align(HAlign::Center)
        })
        .column(PodColumn::Name, "Rate", |c| {
            c.ordering(Ordering::Greater)
                .align(HAlign::Right)
                .width_percent(20)
        });

    layout.add_child(table.with_name("pods").full_screen());

    let pods_panel = Panel::new(layout).title("Pods");

    ui.add_fullscreen_layer(pods_panel);

    ui.add_global_callback(event::Key::Esc, |s| s.select_menubar());

    let sink = ui.cb_sink().clone();

    let (sender, receiver) = kanal::unbounded_async();

    let registry = main_rt.block_on(async move {
        let mut reg = ReflectorRegistry::new(sender, &client);
        reg.register::<Pod>().await;
        reg.register::<Namespace>().await;
        reg
    });

    exchange_rt.spawn(async move {
        let mut stream = receiver.stream();
        while let Some(resource_view) = stream.next().await {
            sink.send(Box::new(|siv| {
                siv.call_on_name("pods", |table: &mut TableView<ResourceView, PodColumn>| {
                    let q = resource_view.gvk();
                    match resource_view {
                        ResourceView::PodView(pod) => {
                            let mut items = table.take_items();
                            items.push(ResourceView::from(pod));
                            table.set_items(items);
                        }
                        ResourceView::NamespaceView(_) => {
                            println!("!");
                        }
                    }
                });
            }))
            .unwrap();
        }
        panic!("!!")
    });

    ui.add_global_callback(event::Key::F5, |s| {
        log::warn!("Or did it?");
    });

    ui.add_global_callback('~', |s| s.toggle_debug_console());

    ui.run();

    Ok(())
}
