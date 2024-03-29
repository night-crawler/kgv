use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use cursive::CursiveRunnable;
use k8s_openapi::api::core::v1::Pod;
use kanal::Sender;
use kube::api::GroupVersionKind;

use crate::backend::k8s_backend::K8sBackend;
use crate::config::args::Args;
use crate::config::extractor::ExtractorConfig;
use crate::config::kgv_configuration::KgvConfiguration;
use crate::eval::engine_factory::build_engine;
use crate::eval::evaluator::Evaluator;
use crate::reexports::sync::Mutex;
use crate::reexports::sync::RwLock;
use crate::theme::get_theme;
use crate::traits::ext::cursive::SivLogExt;
use crate::traits::ext::gvk::GvkStaticExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::ui::command_handler_loop::enter_command_handler_loop;
use crate::ui::components::gvk_switcher::build_gvk_show_chain;
use crate::ui::detail_view_renderer::DetailViewRenderer;
use crate::ui::dispatcher::Dispatcher;
use crate::ui::hotkeys::register_hotkeys;
use crate::ui::resource_manager::ResourceManager;
use crate::ui::signals::{InterUiSignal, ToBackendSignal};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_stack::ViewStack;
use crate::util::watcher::LazyWatcher;

pub(crate) mod backend;
pub(crate) mod config;
pub(crate) mod eval;
pub(crate) mod model;
pub(crate) mod reexports;
pub(crate) mod theme;
pub(crate) mod traits;
pub(crate) mod ui;
pub(crate) mod util;

fn main() -> Result<()> {
    better_panic::install();
    let kgv_configuration = KgvConfiguration::try_from(Args::parse())?;

    // let raw_handle = std::io::stdout().into_raw_mode()?;
    // let orig_hook = panic::take_hook();
    // panic::set_hook(Box::new(move |panic_info| {
    //     raw_handle.suspend_raw_mode().unwrap_or_log();
    //     // otherwise terminal is broken
    //     orig_hook(panic_info);
    //     init_cursive_backend().unwrap_or_log();
    // }));

    let mut ui = CursiveRunnable::default();
    ui.setup_logger(kgv_configuration.logs_dir)?;
    ui.set_theme(get_theme());
    ui.set_fps(1);

    let (from_backend_sender, from_backend_receiver) = kanal::unbounded();
    let (to_backend_sender, from_ui_receiver) = kanal::unbounded();
    let (inter_ui_sender, inter_ui_receiver) = kanal::unbounded();

    let mut backend = K8sBackend::new(
        from_backend_sender.clone(),
        from_ui_receiver,
        kgv_configuration.cache_dir,
        kgv_configuration.num_tokio_backend_threads,
        kgv_configuration.accept_invalid_certs,
    )?;

    backend.spawn_watcher_exchange_task();
    backend.spawn_discovery_task();
    backend.spawn_from_ui_receiver_task();

    register_hotkeys(&mut ui, inter_ui_sender.clone());

    send_init_signals(&to_backend_sender, &inter_ui_sender);

    let extractor_config_watcher = LazyWatcher::new(kgv_configuration.extractor_dirs, |paths| {
        ExtractorConfig::new(paths)
    })?;
    let extractor_config_watcher = Arc::new(extractor_config_watcher);

    let engine_watcher = LazyWatcher::new(kgv_configuration.module_dirs, build_engine)?;
    let engine_watcher = Arc::new(engine_watcher);

    let detail_view_renderer = DetailViewRenderer::new(&engine_watcher, &extractor_config_watcher);
    let resource_manager = ResourceManager::new(
        Evaluator::new(kgv_configuration.num_evaluator_threads, &engine_watcher)?,
        &extractor_config_watcher,
    );

    let store = Arc::new(Mutex::new(UiStore {
        counter: 0,
        view_stack: ViewStack::default(),
        highlighter: Arc::new(ui::highlighter::Highlighter::new("base16-eighties.dark")?),
        selected_gvk: GroupVersionKind::gvk("", "", ""),
        inter_ui_sender: inter_ui_sender.clone(),
        to_backend_sender,
        sink: ui.cb_sink().clone(),
        interactive_command: None,
        gvks: vec![],
        resource_manager: Arc::new(RwLock::new(resource_manager)),
        detail_view_renderer,
        pf_requests: vec![],
    }));

    {
        let dispatcher = Arc::new(Dispatcher::new(from_backend_receiver, store.clone()));
        dispatcher.spawn_n(kgv_configuration.num_dispatcher_threads, "from-backend");
    }

    {
        let dispatcher = Arc::new(Dispatcher::new(inter_ui_receiver, store.clone()));
        dispatcher.spawn_n(kgv_configuration.num_dispatcher_threads, "inter-ui");
    }

    store.spawn_log_updater_thread();

    enter_command_handler_loop(&mut ui, store)?;

    Ok(())
}

pub(crate) fn send_init_signals(
    to_backend_sender: &Sender<ToBackendSignal>,
    ui_to_ui_sender: &Sender<InterUiSignal>,
) {
    let to_backend_sender = to_backend_sender.clone();
    let signal = build_gvk_show_chain(to_backend_sender, &Pod::gvk_for_type());
    ui_to_ui_sender.send_unwrap(signal);
}
