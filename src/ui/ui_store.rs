use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;

use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log::warn;
use cursive::theme::Style;
use cursive::utils::markup::StyledString;
use cursive::views::TextView;
use cursive::Cursive;
use itertools::Itertools;
use kube::api::GroupVersionKind;

use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::reexports::sync::{Mutex, RwLock};
use crate::traits::ext::cursive::SivExt;
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::ui::detail_view_renderer::DetailViewRenderer;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::resource_manager::ResourceManager;
use crate::ui::signals::{InterUiSignal, ToBackendSignal};
use crate::ui::view_meta::{ListViewFilter, ViewMeta};
use crate::ui::view_stack::ViewStack;
use crate::util::panics::ResultExt;
use crate::util::view_with_data::ViewWithMeta;

pub type SinkSender = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

pub struct UiStore {
    pub counter: usize,
    pub highlighter: Arc<crate::ui::highlighter::Highlighter>,
    pub view_stack: ViewStack,

    pub selected_gvk: GroupVersionKind,
    pub inter_ui_sender: kanal::Sender<InterUiSignal>,
    pub to_backend_sender: kanal::Sender<ToBackendSignal>,
    pub sink: SinkSender,

    pub interactive_command: Option<InteractiveCommand>,

    pub resource_manager: Arc<RwLock<ResourceManager>>,

    pub detail_view_renderer: DetailViewRenderer,
    pub gvks: Vec<GroupVersionKind>,
}

impl UiStore {
    pub fn highlight(&self, resource: &ResourceView) -> anyhow::Result<StyledString> {
        let yaml = resource.to_yaml()?;
        self.highlighter.highlight(&yaml, "yaml")
    }

    pub fn highlight_log(&self, log: &str) -> anyhow::Result<StyledString> {
        self.highlighter.highlight(log, "sh")
    }

    pub fn inc_counter(&mut self) -> usize {
        self.counter += 1;
        self.counter
    }

    pub fn should_display_resource(
        &self,
        filter: &ListViewFilter,
        evaluated_resource: &EvaluatedResource,
    ) -> bool {
        evaluated_resource
            .resource
            .namespace()
            .starts_with(&filter.namespace)
            && evaluated_resource.resource.name().contains(&filter.name)
    }

    pub fn get_filtered_resources(&self, view_meta: &ViewMeta) -> Vec<EvaluatedResource> {
        let filter = view_meta.get_filter();
        let gvk = view_meta.get_gvk();
        self.resource_manager
            .read_unwrap()
            .get_resources_iter(gvk)
            .filter(|r| self.should_display_resource(filter, r))
            .cloned()
            .collect()
    }

    pub fn get_filtered_windows(&self, text: &str) -> Vec<(String, Arc<RwLock<ViewMeta>>)> {
        self.view_stack
            .stack
            .iter()
            .map(|view_meta| {
                let title = view_meta.read_unwrap().title();
                (title, Arc::clone(view_meta))
            })
            .filter(|(title, _)| title.to_lowercase().contains(&text.to_lowercase()))
            .collect::<Vec<_>>()
    }

    pub fn get_filtered_gvks(&self, text: &str) -> Vec<(String, GroupVersionKind)> {
        self.gvks
            .iter()
            .map(|gvk| (gvk.full_name(), gvk.clone()))
            .filter(|(name, _)| name.to_lowercase().contains(&text.to_lowercase()))
            .sorted_unstable_by_key(|(name, _)| name.to_string())
            .collect()
    }
}

pub trait UiStoreDispatcherExt {
    fn inc_counter(&self) -> usize;
    fn register_view(&self, view_meta: &ViewWithMeta<ViewMeta>);
    fn spawn_log_updater_thread(&self);
}

impl UiStoreDispatcherExt for Arc<Mutex<UiStore>> {
    fn inc_counter(&self) -> usize {
        self.lock_unwrap().inc_counter()
    }

    fn register_view(&self, view: &ViewWithMeta<ViewMeta>) {
        let meta = Arc::clone(&view.meta);
        let view_name = meta.read_unwrap().get_unique_name();
        self.lock_unwrap().view_stack.push(meta);
        warn!("Registered view: {view_name}");
    }

    fn spawn_log_updater_thread(&self) {
        let store = Arc::clone(self);

        std::thread::Builder::new()
            .name("log-updater".to_string())
            .spawn(move || loop {
                let (sink, log_views, highlighter) = store.get_locking(|store| {
                    (
                        store.sink.clone(),
                        store.view_stack.find_logs(),
                        Arc::clone(&store.highlighter),
                    )
                });

                for log_view in log_views {
                    let mut log_view = log_view.write().unwrap_or_log();

                    match log_view.deref_mut() {
                        ViewMeta::Logs {
                            filter,
                            log_items,
                            next_index,
                            ..
                        } => {
                            let mut highlighted_lines = vec![];
                            let should_clear = *next_index == 0;
                            for log_item in log_items.iter().skip(*next_index) {
                                if log_item.is_placeholder {
                                    break;
                                }

                                let mut line = StyledString::new();

                                if filter.show_timestamps {
                                    line.append(StyledString::styled(
                                        format!("{} ", log_item.timestamp),
                                        Style::secondary(),
                                    ));
                                }

                                line.append(
                                    highlighter
                                        .highlight_substring(&log_item.value, &filter.value, "js")
                                        .unwrap_or_log(),
                                );
                                highlighted_lines.push(line);
                            }

                            if highlighted_lines.is_empty() {
                                continue;
                            }

                            *next_index += highlighted_lines.len();

                            warn!("Going to render {} items", highlighted_lines.len());

                            sink.call_on_name(
                                &log_view.get_unique_name(),
                                move |tv: &mut TextView| {
                                    if should_clear {
                                        tv.set_content("");
                                    }
                                    for h in highlighted_lines {
                                        tv.append(h);
                                    }
                                },
                            );
                        }
                        _ => continue,
                    }
                }

                std::thread::sleep(Duration::from_millis(100));
            })
            .unwrap_or_log();
    }
}
