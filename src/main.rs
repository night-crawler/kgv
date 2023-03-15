use std::sync::{Arc, Mutex};

use anyhow::Result;
use clap::Parser;
use cursive::CursiveRunnable;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use kanal::Sender;
use kube::api::GroupVersionKind;

use crate::backend::k8s_backend::K8sBackend;
use crate::config::args::Args;
use crate::config::extractor::ExtractorConfig;
use crate::config::kgv_configuration::KgvConfiguration;
use crate::eval::engine_factory::build_engine;
use crate::eval::evaluator::Evaluator;
use crate::theme::get_theme;
use crate::traits::ext::cursive::SivLogExt;
use crate::traits::ext::gvk::GvkStaticExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::ui::detail_view_renderer::DetailViewRenderer;
use crate::ui::hotkeys::register_hotkeys;
use crate::ui::loops::{enter_command_handler_loop, spawn_dispatch_events_loop};
use crate::ui::resource_manager::ResourceManager;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::UiStore;
use crate::ui::view_stack::ViewStack;
use crate::util::watcher::LazyWatcher;

pub mod backend;
pub mod config;
pub mod eval;
pub mod model;
pub mod theme;
pub mod traits;
pub mod ui;
pub mod util;

fn main() -> Result<()> {
    let kgv_configuration = KgvConfiguration::try_from(Args::parse())?;

    let mut ui = CursiveRunnable::default();
    ui.setup_logger(kgv_configuration.logs_dir)?;
    ui.set_theme(get_theme());

    let (from_client_sender, from_backend_receiver) = kanal::unbounded();
    let (to_backend_sender, from_ui_receiver) = kanal::unbounded();

    let ui_to_ui_sender = from_client_sender.clone();

    let mut backend = K8sBackend::new(
        from_client_sender,
        from_ui_receiver,
        kgv_configuration.cache_dir,
    )?;

    backend.spawn_watcher_exchange_task();
    backend.spawn_discovery_task();
    backend.spawn_from_ui_receiver_task();

    register_hotkeys(&mut ui, ui_to_ui_sender.clone());

    send_init_signals(&to_backend_sender, &ui_to_ui_sender);

    let extractor_config_watcher = LazyWatcher::new(kgv_configuration.extractor_dirs, |paths| {
        ExtractorConfig::new(paths)
    })?;
    let extractor_config_watcher = Arc::new(extractor_config_watcher);

    let engine_watcher = LazyWatcher::new(kgv_configuration.module_dirs, build_engine)?;
    let engine_watcher = Arc::new(engine_watcher);

    let detail_view_renderer = DetailViewRenderer::new(&engine_watcher, &extractor_config_watcher);
    let resource_manager = ResourceManager::new(
        Evaluator::new(32, &engine_watcher)?,
        &extractor_config_watcher,
    );

    let store = Arc::new(Mutex::new(UiStore {
        counter: 0,
        view_stack: ViewStack::default(),
        highlighter: ui::highlighter::Highlighter::new("base16-eighties.dark")?,
        selected_gvk: GroupVersionKind::gvk("", "", ""),
        to_ui_sender: ui_to_ui_sender,
        to_backend_sender,
        sink: ui.cb_sink().clone(),
        selected_resource: None,
        selected_pod_container: None,
        interactive_command: None,
        resource_manager,
        detail_view_renderer,
    }));

    spawn_dispatch_events_loop(store.clone(), from_backend_receiver.clone());
    spawn_dispatch_events_loop(store.clone(), from_backend_receiver);

    enter_command_handler_loop(&mut ui, store)?;

    Ok(())
}

fn send_init_signals(
    to_backend_sender: &Sender<ToBackendSignal>,
    ui_to_ui_sender: &Sender<ToUiSignal>,
) {
    to_backend_sender.send_unwrap(ToBackendSignal::RequestRegisterGvk(Pod::gvk_for_type()));
    to_backend_sender.send_unwrap(ToBackendSignal::RequestRegisterGvk(
        Namespace::gvk_for_type(),
    ));
    ui_to_ui_sender.send_unwrap(ToUiSignal::ShowGvk(Pod::gvk_for_type()));
    to_backend_sender.send_unwrap(ToBackendSignal::RequestGvkItems(Pod::gvk_for_type()));
}
