use std::sync::{Arc, RwLock};

use cursive::reexports::crossbeam_channel::Sender;
use cursive::Cursive;
use kube::api::GroupVersionKind;

use crate::model::pod::pod_container_view::PodContainerView;
use crate::model::resource::resource_view::EvaluatedResource;
use crate::reexports::Mutex;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::ui::detail_view_renderer::DetailViewRenderer;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::resource_manager::ResourceManager;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::view_meta::{Filter, ViewMeta};
use crate::ui::view_stack::ViewStack;
use crate::util::view_with_data::ViewWithMeta;

pub type SinkSender = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

pub struct UiStore {
    pub counter: usize,
    pub highlighter: crate::ui::highlighter::Highlighter,
    pub view_stack: ViewStack,

    pub selected_gvk: GroupVersionKind,
    pub to_ui_sender: kanal::Sender<ToUiSignal>,
    pub to_backend_sender: kanal::Sender<ToBackendSignal>,
    pub sink: SinkSender,

    pub selected_resource: Option<EvaluatedResource>,
    pub selected_pod_container: Option<PodContainerView>,

    pub interactive_command: Option<InteractiveCommand>,

    pub resource_manager: ResourceManager,

    pub detail_view_renderer: DetailViewRenderer,
}

impl UiStore {
    pub fn inc_counter(&mut self) -> usize {
        self.counter += 1;
        self.counter
    }
    pub fn should_display_resource(
        &self,
        filter: &Filter,
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
            .get_resources_iter(gvk)
            .filter(|r| self.should_display_resource(filter, r))
            .cloned()
            .collect()
    }

    pub fn get_filtered_windows(&self, text: &str) -> Vec<(String, Arc<RwLock<ViewMeta>>)> {
        self
            .view_stack
            .stack
            .iter()
            .map(|view_meta| {
                let title = view_meta.read_unwrap().title();
                (title, Arc::clone(view_meta))
            })
            .filter(|(title, _)| title.contains(text))
            .collect::<Vec<_>>()
    }
}

pub trait UiStoreDispatcherExt {
    fn inc_counter(&self) -> usize;
    fn register_view(&self, view_meta: &ViewWithMeta<ViewMeta>);
}

impl UiStoreDispatcherExt for Arc<Mutex<UiStore>> {
    fn inc_counter(&self) -> usize {
        self.lock_unwrap().inc_counter()
    }

    fn register_view(&self, view: &ViewWithMeta<ViewMeta>) {
        let meta = Arc::clone(&view.meta);
        self.lock_unwrap().view_stack.push(meta)
    }
}
