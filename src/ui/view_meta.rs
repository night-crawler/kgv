use chrono::FixedOffset;
use kube::api::GroupVersionKind;
use strum_macros::AsRefStr;

use crate::model::log_request::LogRequest;
use crate::traits::ext::gvk::GvkNameExt;
use crate::util::error::{LogError, LogErrorOptionExt, LogErrorResultExt};
use crate::util::panics::OptionExt;

#[derive(Debug, Default, Clone, Hash)]
pub(crate) struct ListViewFilter {
    pub(crate) namespace: String,
    pub(crate) name: String,
}

impl ListViewFilter {
    pub(crate) fn is_empty(&self) -> bool {
        self.namespace.is_empty() && self.name.is_empty()
    }
}

#[derive(Debug, Default, Clone, Hash)]
pub(crate) struct LogFilter {
    pub(crate) value: String,
    pub(crate) show_timestamps: bool,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct LogItem {
    pub(crate) seq_id: usize,
    pub(crate) timestamp: chrono::DateTime<FixedOffset>,
    pub(crate) value: String,
    pub(crate) is_placeholder: bool,
}

impl LogItem {
    pub(crate) fn new(seq_id: usize, raw_log_data: Vec<u8>) -> Result<Self, LogError> {
        let raw_log_string = String::from_utf8(raw_log_data)
            .to_log_error(|err| format!("Failed to parse string: {err}"))?;
        let (timestamp, log_string) = raw_log_string
            .split_once(' ')
            .to_log_warn(|| "Raw log data had no space".to_string())?;

        let timestamp = chrono::DateTime::parse_from_rfc3339(timestamp)
            .to_log_error(|err| format!("Failed to parse timestamp: {err}"))?;
        Ok(LogItem {
            timestamp,
            value: log_string.to_string(),
            seq_id,
            is_placeholder: false,
        })
    }
}

#[derive(Debug, AsRefStr)]
pub(crate) enum ViewMeta {
    List {
        id: usize,
        gvk: GroupVersionKind,
        filter: ListViewFilter,
    },
    Details {
        id: usize,
        gvk: GroupVersionKind,
        name: String,
        uid: String,
    },
    Code {
        id: usize,
        gvk: GroupVersionKind,
        title: String,
        uid: String,
    },
    Logs {
        id: usize,
        filter: LogFilter,
        request: LogRequest,
        log_items: Vec<LogItem>,
        next_index: usize,
    },
    Dialog {
        id: usize,
        name: String,
    },
    WindowSwitcher {
        id: usize,
    },
    GvkSwitcher {
        id: usize,
    },
}

impl ViewMeta {
    pub(crate) fn title(&self) -> String {
        let title = format!("{: >4} {: >7}", self.get_id(), self.as_ref());
        let unique_part = match self {
            ViewMeta::List { gvk, filter, .. } => {
                let mut repr = gvk.full_name();
                if !filter.is_empty() {
                    repr.push_str(" (");
                    if !filter.namespace.is_empty() {
                        repr.push_str(&format!("namespace = {}", filter.namespace));
                    }
                    if !filter.name.is_empty() {
                        repr.push_str(&format!("name = {}", filter.name));
                    }
                    repr.push(')');
                }
                repr
            }
            ViewMeta::Details { gvk, name, .. } => {
                format!("{} {name}", gvk.full_name())
            }
            ViewMeta::Code { gvk, title, .. } => {
                format!("{} {title}", gvk.full_name())
            }
            ViewMeta::Dialog { name, .. } => name.clone(),
            ViewMeta::WindowSwitcher { .. } => "Window Switcher".to_string(),
            ViewMeta::GvkSwitcher { .. } => "Gvk Switcher".to_string(),
            ViewMeta::Logs {
                filter, request, ..
            } => {
                let mut repr = format!(
                    "{}/{}/{}",
                    request.namespace,
                    request.pod_name,
                    request.log_params.container.as_ref().unwrap_or_log()
                );
                if !filter.value.is_empty() {
                    repr.push_str(&format!(" (value = {})", filter.value));
                }
                repr
            }
        };

        format!("{title} - {unique_part}")
    }
    pub(crate) fn get_unique_name(&self) -> String {
        match self {
            ViewMeta::List { id, gvk, filter: _ } => {
                format!("gvk-list-{id}-{}-table", gvk.full_name())
            }
            ViewMeta::Details { id, gvk, uid, .. } => {
                format!("gvk-details-{id}-{}-{uid}", gvk.full_name())
            }
            ViewMeta::Code { id, gvk, uid, .. } => {
                format!("gvk-code-view-{id}-{}-{uid}", gvk.full_name())
            }
            ViewMeta::Dialog { id, name } => format!("dialog-{id}-{name}"),
            ViewMeta::WindowSwitcher { id } => format!("window-switcher-list-{id}"),
            ViewMeta::GvkSwitcher { id } => format!("gvk-switcher-list-{id}"),
            ViewMeta::Logs { id, .. } => format!("logs-{id}"),
        }
    }

    pub(crate) fn is_list(&self) -> bool {
        matches!(self, ViewMeta::List { .. })
    }

    pub(crate) fn get_uid(&self) -> Option<String> {
        match self {
            ViewMeta::Details { uid, .. } | ViewMeta::Code { uid, .. } => Some(uid.to_string()),
            _ => None,
        }
    }

    pub(crate) fn get_edit_name(&self, edit_type: &str) -> String {
        format!("{}-edit-{edit_type}", self.get_unique_name())
    }

    pub(crate) fn get_checkbox_name(&self, checkbox_type: &str) -> String {
        format!("{}-checkbox-{checkbox_type}", self.get_unique_name())
    }

    pub(crate) fn get_panel_name(&self) -> String {
        format!("{}-panel", self.get_unique_name())
    }

    pub(crate) fn set_namespace(&mut self, namespace: String) {
        match self {
            ViewMeta::List { filter, .. } => filter.namespace = namespace,
            this => panic!("Setting namespace {namespace} on {:?}", this),
        }
    }

    pub(crate) fn set_name(&mut self, name: String) {
        match self {
            ViewMeta::List { filter, .. } => filter.name = name,
            this => panic!("Setting name {name} on {:?}", this),
        }
    }

    pub(crate) fn get_id(&self) -> usize {
        match self {
            Self::List { id, .. }
            | Self::Details { id, .. }
            | Self::Dialog { id, .. }
            | Self::Code { id, .. }
            | Self::GvkSwitcher { id, .. }
            | Self::WindowSwitcher { id }
            | Self::Logs { id, .. } => *id,
        }
    }

    pub(crate) fn get_filter(&self) -> &ListViewFilter {
        match self {
            ViewMeta::List { filter, .. } => filter,
            this => panic!("Trying to get filter on {:?}", this),
        }
    }

    pub(crate) fn get_gvk(&self) -> &GroupVersionKind {
        match self {
            ViewMeta::List { gvk, .. }
            | ViewMeta::Code { gvk, .. }
            | ViewMeta::Details { gvk, .. } => gvk,
            this => panic!("{:?} does not have GVK", this),
        }
    }
}

pub(crate) trait ViewMetaLogExt {
    fn get_log_filter(&self) -> &LogFilter;
    fn get_log_filter_clearing_mut(&mut self) -> &mut LogFilter;
    fn get_log_request(&self) -> &LogRequest;
    fn get_log_request_clearing_mut(&mut self) -> &mut LogRequest;
    fn push_log_item(&mut self, item: LogItem);
    fn set_log_since_seconds(&mut self, num_minutes: usize);
    fn set_log_tail_lines(&mut self, num_lines: usize);
    fn set_log_show_previous(&mut self, show_previous: bool);
    fn set_log_search_text(&mut self, text: String);
    fn set_log_show_timestamps(&mut self, show: bool);
}

impl ViewMetaLogExt for ViewMeta {
    fn get_log_filter(&self) -> &LogFilter {
        match self {
            ViewMeta::Logs { filter, .. } => filter,
            this => panic!("{:?} is not Logs", this),
        }
    }

    fn get_log_filter_clearing_mut(&mut self) -> &mut LogFilter {
        match self {
            ViewMeta::Logs {
                filter, next_index, ..
            } => {
                *next_index = 0;
                filter
            }
            this => panic!("{:?} is not Logs", this),
        }
    }

    fn get_log_request(&self) -> &LogRequest {
        match self {
            ViewMeta::Logs { request, .. } => request,
            this => panic!("{:?} is not Logs", this),
        }
    }

    fn get_log_request_clearing_mut(&mut self) -> &mut LogRequest {
        match self {
            ViewMeta::Logs {
                request,
                log_items,
                next_index,
                ..
            } => {
                *next_index = 0;
                log_items.clear();
                request
            }
            this => panic!("{:?} is not Logs", this),
        }
    }

    fn push_log_item(&mut self, item: LogItem) {
        match self {
            ViewMeta::Logs { log_items, .. } => {
                let seq_id = item.seq_id;
                while log_items.len() <= seq_id {
                    log_items.push(LogItem::default())
                }

                log_items[seq_id] = item;
            }
            this => panic!("{:?} is not Logs", this),
        }
    }

    fn set_log_since_seconds(&mut self, num_minutes: usize) {
        let num_seconds = Some((num_minutes + 60) as i64);
        if self.get_log_request().log_params.since_seconds == num_seconds {
            return;
        }
        self.get_log_request_clearing_mut().log_params.since_seconds = num_seconds;
    }

    fn set_log_tail_lines(&mut self, num_lines: usize) {
        let num_lines = Some(num_lines as i64);
        if self.get_log_request().log_params.tail_lines == num_lines {
            return;
        }
        self.get_log_request_clearing_mut().log_params.tail_lines = num_lines;
    }

    fn set_log_show_previous(&mut self, show_previous: bool) {
        if self.get_log_request().log_params.previous == show_previous {
            return;
        }
        self.get_log_request_clearing_mut().log_params.previous = show_previous;
    }

    fn set_log_search_text(&mut self, text: String) {
        if self.get_log_filter().value == text {
            return;
        }
        self.get_log_filter_clearing_mut().value = text;
    }

    fn set_log_show_timestamps(&mut self, show: bool) {
        if self.get_log_filter().show_timestamps == show {
            return;
        }
        self.get_log_filter_clearing_mut().show_timestamps = show;
    }
}
