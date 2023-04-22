use std::sync::Arc;

use cursive::direction::Orientation;
use cursive::reexports::log::error;
use cursive::traits::Nameable;
use cursive::views::{Dialog, EditView, LinearLayout, Panel};
use cursive::Cursive;
use k8s_openapi::api::core::v1::{Container, Pod};
use kube::ResourceExt;

use crate::model::port_forward_request::PortForwardRequest;
use crate::reexports::sync::Mutex;
use crate::traits::ext::cloning_callback::CloningCallbackExt;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::pod::PodExt;
use crate::ui::signals::ToBackendSignal;
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::ViewMeta;
use crate::util::panics::{OptionExt, ResultExt};
use crate::util::view_with_data::ViewWithMeta;

fn get_first_container_port(container: &Container) -> Option<u16> {
    container
        .ports
        .as_ref()?
        .iter()
        .next()
        .map(|port| port.container_port as u16)
}

fn get_first_pod_port(pod: &Pod) -> Option<u16> {
    pod.get_pod_containers()?
        .into_iter()
        .filter_map(|pod| get_first_container_port(&pod.container))
        .next()
}

pub(crate) fn build_port_forwarding_dialog_view(
    pod: &Pod,
    container: &Container,
    store: Arc<Mutex<UiStore>>,
) -> anyhow::Result<ViewWithMeta<ViewMeta>> {
    let default_pod_port = get_first_container_port(container)
        .or_else(|| get_first_pod_port(pod))
        .unwrap_or(80);

    let default_host_port = default_pod_port.saturating_add(10000);

    let (to_backend_sender, counter) =
        store.locking(|mut store| Ok((store.to_backend_sender.clone(), store.inc_counter())))?;

    let view_meta = ViewMeta::Dialog {
        id: counter,
        name: format!("Port Forward for {}", pod.name_any()),
    };

    let host_port_edit_name = view_meta.get_edit_name("host_port");
    let pod_port_edit_name = view_meta.get_edit_name("pod_port");
    let host_edit_name = view_meta.get_edit_name("host");

    let submit = {
        let namespace = pod.namespace().unwrap_or_default();
        let pod_name = pod.name_any();
        let host_port_edit_name = host_port_edit_name.clone();
        let pod_port_edit_name = pod_port_edit_name.clone();
        let host_edit_name = host_edit_name.clone();

        Arc::new(move |siv: &mut Cursive| {
            let host_port = siv
                .call_on_name(&host_port_edit_name, |view: &mut EditView| {
                    view.get_content()
                })
                .unwrap_or_log()
                .parse::<u16>();

            let host_port = if let Ok(host_port) = host_port {
                host_port
            } else {
                error!("Invalid host port: {:?}", host_port);
                return;
            };

            if store
                .lock_unwrap()
                .pf_requests
                .iter()
                .any(|req| req.host_port == host_port)
            {
                error!("Port {host_port} is already in use");
                return;
            }

            let pod_port = siv
                .call_on_name(&pod_port_edit_name, |view: &mut EditView| {
                    view.get_content()
                })
                .unwrap_or_log()
                .parse::<u16>();

            let pod_port = if let Ok(pod_port) = pod_port {
                pod_port
            } else {
                error!("Invalid pod port: {:?}", pod_port);
                return;
            };

            let host = siv
                .call_on_name(&host_edit_name, |view: &mut EditView| view.get_content())
                .unwrap_or_log();

            let namespace = namespace.clone();
            let pod_name = pod_name.clone();

            let request = PortForwardRequest {
                id: counter,
                namespace,
                pod_name,
                host_port,
                pod_port,
                host: host.as_ref().to_string(),
            };

            to_backend_sender
                .send(ToBackendSignal::PortForward(Arc::new(request)))
                .unwrap_or_log();

            let mut store = store.lock_unwrap();
            store.view_stack.pop();
            siv.pop_layer();
        })
    };

    let host_port_edit_view = submit.cloning(|submit| {
        EditView::new()
            .content(format!("{default_host_port}"))
            .on_submit(move |siv, _| submit(siv))
            .with_name(&host_port_edit_name)
    });

    let pod_port_edit_view = submit.cloning(|submit| {
        EditView::new()
            .content(format!("{default_pod_port}"))
            .on_submit(move |siv, _| submit(siv))
            .with_name(&pod_port_edit_name)
    });

    let host_edit_view = submit.cloning(|submit| {
        EditView::new()
            .content("127.0.0.1")
            .on_submit(move |siv, _| submit(siv))
            .with_name(&host_edit_name)
    });

    let mut main_layout = LinearLayout::new(Orientation::Vertical);
    main_layout.add_child(Panel::new(host_port_edit_view).title("Host Port"));
    main_layout.add_child(Panel::new(pod_port_edit_view).title("Pod Port"));
    main_layout.add_child(Panel::new(host_edit_view).title("Host"));

    let panel = Dialog::around(main_layout)
        .title(format!(
            "Port Forwarding: {}/{}",
            pod.name_any(),
            container.name
        ))
        .button("Forward", move |siv| {
            submit(siv);
        });

    Ok(ViewWithMeta::new(panel, view_meta))
}
