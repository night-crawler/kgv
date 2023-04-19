use std::sync::Arc;

use cursive::direction::Orientation;
use cursive::reexports::log::error;
use cursive::traits::{Nameable, Resizable, Scrollable};
use cursive::view::ScrollStrategy;
use cursive::views::{Checkbox, LinearLayout, Panel};
use cursive_cached_text_view::CachedTextView;
use k8s_openapi::api::core::v1::{Container, Pod};
use kube::api::LogParams;
use kube::ResourceExt;

use crate::model::log_request::LogRequest;
use crate::reexports::sync::Mutex;
use crate::traits::ext::cloning_callback::CloningCallbackExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::ui::signals::InterUiSignal;
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::{LogFilter, ViewMeta};
use crate::util::ui::build_edit_view;
use crate::util::view_with_data::ViewWithMeta;

pub fn build_log_view(
    pod: &Pod,
    container: &Container,
    store: Arc<Mutex<UiStore>>,
) -> anyhow::Result<ViewWithMeta<ViewMeta>> {
    let (to_ui_sender, counter) =
        store.locking(|mut store| Ok((store.inter_ui_sender.clone(), store.inc_counter())))?;

    let log_params = LogParams {
        container: Some(container.name.clone()),
        follow: true,
        limit_bytes: None,
        pretty: true,
        previous: false,
        since_seconds: Some(60 * 60),
        tail_lines: Some(1000),
        timestamps: true,
    };

    let request = LogRequest {
        id: counter,
        namespace: pod.namespace().unwrap_or_default(),
        pod_name: pod.name_any(),
        log_params,
    };

    let view_meta = ViewMeta::Logs {
        id: counter,
        filter: LogFilter {
            show_timestamps: true,
            value: String::default(),
        },
        request,
        log_items: vec![],
        next_index: 0,
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);
    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);

    let filter_edit_view = to_ui_sender.cloning(|to_ui_sender| {
        build_edit_view(view_meta.get_edit_name("filter"), "", move |_, text, _| {
            to_ui_sender.send_unwrap(InterUiSignal::LogsApplyHighlight(counter, text.into()));
        })
    });
    let filter_edit_view_panel = Panel::new(filter_edit_view).title("Search").full_width();

    let since_minutes_edit_view = to_ui_sender.cloning(|to_ui_sender| {
        build_edit_view(
            view_meta.get_edit_name("since_minutes"),
            "60",
            move |_, text, _| {
                if let Ok(value) = text.parse::<usize>() {
                    to_ui_sender.send_unwrap(InterUiSignal::LogsApplySinceMinutes(counter, value));
                } else {
                    error!("Failed to parse minutes: {}", text);
                }
            },
        )
    });
    let since_minutes_panel = Panel::new(since_minutes_edit_view).title("Since minutes");

    let filter_tail_lines_edit_view = to_ui_sender.cloning(|to_ui_sender| {
        build_edit_view(
            view_meta.get_edit_name("tail_lines"),
            "1000",
            move |_, text, _| {
                if let Ok(value) = text.parse::<usize>() {
                    to_ui_sender.send_unwrap(InterUiSignal::LogsApplyTailLines(counter, value));
                } else {
                    error!("Failed to parse tail lines: {}", text);
                }
            },
        )
    });
    let filter_tail_lines_panel = Panel::new(filter_tail_lines_edit_view).title("Tail lines");

    let cb_timestamps = to_ui_sender.cloning(|to_ui_sender| {
        Checkbox::new()
            .on_change(move |_, checked| {
                to_ui_sender.send_unwrap(InterUiSignal::LogsApplyTimestamps(counter, checked));
            })
            .checked()
            .with_name(view_meta.get_checkbox_name("timestamps"))
    });
    let cb_timestamps_panel = Panel::new(cb_timestamps).title("Timestamps");

    let cb_previous = to_ui_sender.cloning(|to_ui_sender| {
        Checkbox::new()
            .on_change(move |_, checked| {
                to_ui_sender.send_unwrap(InterUiSignal::LogsApplyPrevious(counter, checked));
            })
            .with_name(view_meta.get_checkbox_name("previous"))
    });
    let cb_previous_panel = Panel::new(cb_previous).title("Previous");

    filter_layout.add_child(filter_edit_view_panel);
    filter_layout.add_child(cb_timestamps_panel);
    filter_layout.add_child(cb_previous_panel);
    filter_layout.add_child(since_minutes_panel);
    filter_layout.add_child(filter_tail_lines_panel);

    let tv = CachedTextView::new("", 5)
        // .no_wrap()
        .with_name(view_meta.get_unique_name())
        .full_screen()
        .scrollable()
        .scroll_x(true)
        .scroll_y(true)
        .scroll_strategy(ScrollStrategy::StickToBottom);

    main_layout.add_child(filter_layout.full_width());

    let dialog_title = format!(
        "Logs {}/{}/{}",
        pod.namespace().unwrap_or_default(),
        pod.name_any(),
        container.name
    );

    main_layout.add_child(tv);

    let panel = Panel::new(main_layout)
        .title(dialog_title)
        .with_name(view_meta.get_panel_name());

    Ok(ViewWithMeta::new(panel, view_meta))
}
