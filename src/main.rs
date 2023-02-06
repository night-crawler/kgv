use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use cursive::direction::Orientation;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::Debug;
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive::{event, menu, CursiveRunnable, Cursive};
use cursive_table_view::{TableView, TableViewItem};
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use k8s_openapi::Resource;
use kube::api::GroupVersionKind;
use kube::Client;

use crate::client::{discover_gvk, ReflectorRegistry, ResourceView};
use crate::util::ui::group_gvks;

pub mod client;
pub mod theme;
pub mod util;

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
                }
                s.pop_layer();
            });
        }
        menu_bar.add_subtree(group_name, group_tree);
    }
}

fn build_main_layout() -> LinearLayout {
    let mut main_layout = LinearLayout::new(Orientation::Vertical);

    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);
    filter_layout.add_child(Panel::new(EditView::new()).title("Namespaces").full_width());
    filter_layout.add_child(Panel::new(EditView::new()).title("Name").full_width());

    let table: TableView<ResourceView, PodColumn> = TableView::new()
        .column(PodColumn::Namespace, "Namespace", |c| c)
        .column(PodColumn::Name, "Name", |c| c)
        .column(PodColumn::Name, "Rate", |c| c);

    let table_panel = Panel::new(table.with_name("pods").full_screen()).title("Pods");

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

    let gvks = main_rt.block_on(async { discover_gvk(&client).await })?;

    let mut ui = CursiveRunnable::default();
    let mut ui = ui.runner();

    let current_gvk = Arc::new(Mutex::new(GroupVersionKind::gvk(
        Pod::GROUP,
        Pod::VERSION,
        Pod::KIND,
    )));

    build_menu(ui.menubar(), gvks, &current_gvk);
    ui.set_autohide_menu(false);

    cursive::logger::init();
    log::set_max_level(Debug);

    let main_layout = build_main_layout();
    ui.add_fullscreen_layer(main_layout);

    let sink = ui.cb_sink().clone();

    let (sender, receiver) = kanal::unbounded_async();

    let registry = main_rt.block_on(async {
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
                })
                .expect("!!!!!!!asd");
            }))
            .expect("????");
        }
        panic!("@@@@@@@@@@@@@@@@@@@@@@@@@!!")
    });


    ui.add_global_callback('~', |s| s.toggle_debug_console());
    ui.add_global_callback(event::Key::Esc, |s| s.select_menubar());

    ui.run();

    Ok(())
}
