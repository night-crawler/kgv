use std::sync::Arc;

use cursive::direction::Orientation;
use cursive::traits::Nameable;
use cursive::views::{Dialog, LinearLayout, SelectView};

use crate::model::port_forward_request::PortForwardRequest;
use crate::reexports::sync::Mutex;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::ui::signals::ToBackendSignal;
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::ViewMeta;
use crate::util::view_with_data::ViewWithMeta;

pub(crate) fn build_port_forwarding_view(
    store: Arc<Mutex<UiStore>>,
) -> anyhow::Result<ViewWithMeta<ViewMeta>> {
    let (pf_requests, to_backend_sender, counter) = store.locking(|mut store| {
        Ok((
            store.pf_requests.clone(),
            store.to_backend_sender.clone(),
            store.inc_counter(),
        ))
    })?;

    let view_meta = ViewMeta::Dialog {
        id: counter,
        name: "Port Forwarding List".to_string(),
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);

    let mut sv = SelectView::new();
    for request in pf_requests {
        let title = format!(
            "{}:{} -> {}:{}",
            request.host, request.host_port, request.pod_name, request.pod_port
        );
        sv.add_item(title, request);
    }

    let sv_name = view_meta.get_unique_name();
    sv.set_on_submit(
        move |siv: &mut cursive::Cursive, request: &Arc<PortForwardRequest>| {
            to_backend_sender.send_unwrap(ToBackendSignal::StopForwarding(Arc::clone(request)));
            let mut store = store.lock_unwrap();
            store.pf_requests.retain(|req| req.id != request.id);

            siv.call_on_name(&sv_name, |sv: &mut SelectView<Arc<PortForwardRequest>>| {
                let index =
                    if let Some(index) = sv.iter().position(|(_, item)| item.id == request.id) {
                        index
                    } else {
                        return;
                    };
                sv.remove_item(index);
            });
        },
    );

    main_layout.add_child(sv.with_name(view_meta.get_unique_name()));

    let panel = Dialog::around(main_layout).title("Port Forwarding List");
    Ok(ViewWithMeta::new(panel, view_meta))
}
