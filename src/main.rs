use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::{Arc, LockResult, Mutex};
use std::thread;

use cursive::align::HAlign;
use cursive::direction::Orientation;
use cursive::traits::*;
use cursive::view::scroll;
use cursive::views::{Dialog, DummyView, LinearLayout, Menubar, Panel, ResizedView};
use cursive::{event, menu, CursiveRunnable, Printer, Cursive};
use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log;
use cursive_table_view::{TableView, TableViewItem};
use rand::{thread_rng, Rng};
use crate::client::bla;
use anyhow::Result;
pub mod theme;
pub mod client;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum PodColumn {
    Name,
    Count,
    Rate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PodView {
    name: String,
    count: usize,
    rate: usize,
}

impl TableViewItem<PodColumn> for Arc<Mutex<PodView>> {
    fn to_column(&self, column: PodColumn) -> String {
        match self.lock() {
            Ok(pod_guard) => pod_guard.to_column(column),
            Err(err) => panic!("Error {}", err),
        }
    }

    fn cmp(&self, other: &Self, column: PodColumn) -> Ordering
    where
        Self: Sized,
    {
        match (self.lock(), other.lock()) {
            (Ok(self_pod_guard), Ok(other_pod_guard)) => {
                self_pod_guard.cmp(&*other_pod_guard, column)
            }
            _ => panic!("Error"),
        }
    }
}

impl TableViewItem<PodColumn> for PodView {
    fn to_column(&self, column: PodColumn) -> String {
        match column {
            PodColumn::Name => self.name.to_string(),
            PodColumn::Count => format!("{}", self.count),
            PodColumn::Rate => format!("{}", self.rate),
        }
    }

    fn cmp(&self, other: &Self, column: PodColumn) -> Ordering
    where
        Self: Sized,
    {
        match column {
            PodColumn::Name => self.name.cmp(&other.name),
            PodColumn::Count => self.count.cmp(&other.count),
            PodColumn::Rate => self.rate.cmp(&other.rate),
        }
    }
}

#[derive(Default)]
struct GlobalState {
    pods: BTreeMap<String, Arc<Mutex<PodView>>>,
}


fn main() -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let a = rt.block_on(async move {
        // sender.send(()).await.unwrap();
        bla().await
    })?;

    return Ok(());

    let (sender, receiver) = kanal::unbounded_async::<()>();

    let mut ui = CursiveRunnable::default();
    let mut ui = ui.runner();

    ui.menubar()
        .add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    ui.set_autohide_menu(false);
    cursive::logger::init();

    let mut layout = LinearLayout::new(Orientation::Horizontal);

    let table: TableView<PodView, PodColumn> = TableView::new().column(PodColumn::Name, "Name", |c| c.width_percent(20))
        .column(PodColumn::Count, "Count", |c| c.align(HAlign::Center))
        .column(PodColumn::Rate, "Rate", |c| {
            c.ordering(Ordering::Greater)
                .align(HAlign::Right)
                .width_percent(20)
        });

    layout.add_child(table.with_name("pods").full_screen());

    // layout.add_child(Lol::default());

    // layout.add_child(t1.min_size((32, 20)));
    // layout.add_child(ResizedView::with_fixed_size((4, 0), DummyView));
    // layout.add_child(create_table(container.clone()).min_size((32, 20)));

    let pods_panel = Panel::new(layout).title("Pods");
    // ui.add_layer(pods_panel);

    // ui.add_layer(Dialog::around(layout).title("Table View Demo"));

    ui.add_fullscreen_layer(pods_panel);

    ui.add_global_callback(event::Key::Esc, |s| s.select_menubar());

    let sink: Sender<Box<dyn FnOnce(&mut Cursive) + Send>> = ui.cb_sink().clone();




    ui.add_global_callback(event::Key::F5, |s| {
        log::warn!("Or did it?");
    });

    ui.add_global_callback('~', |s| s.toggle_debug_console());

    ui.run();

    Ok(())
}

fn create_table(state: Arc<Mutex<GlobalState>>) -> TableView<Arc<Mutex<PodView>>, PodColumn> {
    let pods = match state.lock() {
        Ok(mut state) => {
            state.pods.insert(
                "a".to_string(),
                Arc::new(Mutex::new(PodView {
                    name: "a".to_string(),
                    count: 0,
                    rate: 0,
                })),
            );
            state.pods.values().map(Arc::clone).collect::<Vec<_>>()
        }
        Err(err) => panic!("Error {}", err),
    };

    let a = TableView::<Arc<Mutex<PodView>>, PodColumn>::new()
        .column(PodColumn::Name, "Name", |c| c.width_percent(20))
        .column(PodColumn::Count, "Count", |c| c.align(HAlign::Center))
        .column(PodColumn::Rate, "Rate", |c| {
            c.ordering(Ordering::Greater)
                .align(HAlign::Right)
                .width_percent(20)
        })
        .items(pods);
    a
}
