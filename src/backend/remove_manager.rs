use cursive::reexports::log::{error, warn};
use itertools::Either;
use kube::api::{DeleteParams, DynamicObject};
use kube::core::response::StatusSummary;
use kube::discovery::pinned_kind;
use kube::ResourceExt;
use kube::{Api, Client};

use crate::model::dynamic_object::DynamicObjectWrapper;
use crate::model::resource::resource_view::ResourceView;
use crate::traits::ext::gvk::GvkExt;
use crate::traits::ext::gvk::GvkNameExt;
use crate::ui::signals::FromBackendSignal;

pub(crate) struct RemoveManager {
    client: Client,
    from_backend_sender: kanal::AsyncSender<FromBackendSignal>,
}

impl RemoveManager {
    pub(crate) fn new(
        client: &Client,
        from_backend_sender: kanal::AsyncSender<FromBackendSignal>,
    ) -> Self {
        Self {
            client: client.clone(),
            from_backend_sender,
        }
    }

    pub(crate) async fn remove(&self, resource: ResourceView) -> anyhow::Result<()> {
        let gvk = resource.gvk();
        warn!(
            "Removing resource: {}, name: {}",
            gvk.full_name(),
            resource.full_unique_name()
        );
        let (ar, _caps) = pinned_kind(&self.client, &gvk).await?;

        let api = if resource.namespace().is_empty() {
            Api::<DynamicObject>::all_with(self.client.clone(), &ar)
        } else {
            Api::<DynamicObject>::namespaced_with(self.client.clone(), &resource.namespace(), &ar)
        };

        let params = DeleteParams::default();

        let result = api.delete(&resource.name(), &params).await?;
        match result {
            Either::Left(dynamic_object) => {
                warn!(
                    "Removed resource: {}; timestamp: {:?}",
                    dynamic_object.name_any(),
                    dynamic_object.metadata.deletion_timestamp
                );
                let wrapper = DynamicObjectWrapper(dynamic_object, gvk);
                let deleted_resource = ResourceView::DynamicObject(wrapper.into());
                self.from_backend_sender
                    .send(FromBackendSignal::ResourceDeleted(deleted_resource))
                    .await?;
            }
            Either::Right(status) => {
                if status.status == Some(StatusSummary::Success) {
                    warn!(
                        "Removed resource (status success): {}",
                        resource.full_unique_name()
                    );
                    return Ok(());
                }
                error!(
                    "Failed to remove resource {}: {:?}",
                    resource.full_unique_name(),
                    status
                )
            }
        }

        Ok(())
    }
}
