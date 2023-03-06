use std::sync::{Arc, Mutex};

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
use crate::util::panics::ResultExt;

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
