use std::sync::{Arc, Mutex, RwLock};

use cursive::theme::BaseColor;
use cursive::traits::*;
use cursive::utils::markup::StyledString;
use cursive::views::{LinearLayout, Panel};
use cursive_table_view::TableView;
use strum::IntoEnumIterator;

use crate::model::pod::pod_container_column::PodContainerColumn;
use crate::model::pod::pod_container_view::PodContainerView;
use crate::traits::ext::gvk::{GvkExt, GvkNameExt};
use crate::ui::signals::ToUiSignal;
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::ViewMeta;
use crate::util::panics::{OptionExt, ResultExt};
use crate::util::view_with_data::ViewWithMeta;

pub fn build_pod_detail_layout(store: Arc<Mutex<UiStore>>) -> Option<ViewWithMeta<ViewMeta>> {
    let mut main_layout = LinearLayout::vertical();
    let counter = store.inc_counter();
    let (panel_title, view_meta) = {
        let store = store.lock().unwrap_or_log();
        if let Some(evaluated_resource) = store.selected_resource.as_ref() {
            let view_meta = ViewMeta::PodDetail {
                id: counter,
                uid: evaluated_resource.resource.uid().unwrap_or_log(),
            };

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
            (styled, view_meta)
        } else {
            return None;
        }
    };

    let mut table: TableView<PodContainerView, PodContainerColumn> = TableView::new();
    {
        let store = store.clone();
        let name = view_meta.get_unique_name();
        table.set_on_submit(move |siv, _row, index| {
            siv.call_on_name(
                &name,
                |table: &mut TableView<PodContainerView, PodContainerColumn>| {
                    let mut store = store.lock().unwrap_or_log();
                    store.selected_pod_container = table.borrow_item(index).cloned();
                    store
                        .to_ui_sender
                        .send(ToUiSignal::ExecuteCurrent)
                        .unwrap_or_log();
                },
            );
        });
    }

    {
        let name = view_meta.get_unique_name();
        table.set_on_select(move |siv, _row, index| {
            siv.call_on_name(
                &name,
                |table: &mut TableView<PodContainerView, PodContainerColumn>| {
                    store.lock().unwrap_or_log().selected_pod_container =
                        table.borrow_item(index).cloned();
                },
            );
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

    let table = table.with_name(&view_meta.get_unique_name()).full_screen();

    let panel = Panel::new(table).title(panel_title);
    main_layout.add_child(panel);

    Some(ViewWithMeta {
        inner: Box::new(main_layout),
        meta: Arc::new(RwLock::new(view_meta)),
    })
}
