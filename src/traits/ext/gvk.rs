use itertools::Itertools;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kube::api::GroupVersionKind;
use kube::ResourceExt;
use std::fmt::Debug;
use crate::model::pseudo_resource::PSEUDO_RESOURCE_JOIN_SEQ;

pub trait GvkNameExt {
    fn full_name(&self) -> String;
    fn short_name(&self) -> String;
}

impl GvkNameExt for GroupVersionKind {
    fn full_name(&self) -> String {
        [&self.group, &self.version, &self.kind]
            .iter()
            .filter(|part| !part.is_empty())
            .join("/")
    }

    fn short_name(&self) -> String {
        format!("{}/{}", &self.version, &self.kind)
    }
}

pub trait GvkStaticExt {
    fn gvk_for_type() -> GroupVersionKind;
}

pub trait GvkExt {
    fn gvk(&self) -> GroupVersionKind;
}

pub trait PseudoResourceGvkExt {
    fn get_pseudo_parent(&self) -> Option<GroupVersionKind>;
}

pub trait PseudoGvkBuilderExt {
    fn build_pseudo_gvk(&self, extractor_name: &str) -> GroupVersionKind;
}

impl PseudoResourceGvkExt for GroupVersionKind {
    fn get_pseudo_parent(&self) -> Option<GroupVersionKind> {
        if let Some((left, _)) = self.kind.rsplit_once('/') {
            let mut gvk = self.clone();
            gvk.kind = left.to_string();
            Some(gvk)
        } else {
            None
        }
    }
}

impl<T> GvkStaticExt for T
where
    T: Metadata<Ty = ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
{
    fn gvk_for_type() -> GroupVersionKind {
        GroupVersionKind::gvk(T::GROUP, T::VERSION, T::KIND)
    }
}

impl<T> GvkExt for T
where
    T: Metadata<Ty = ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
{
    fn gvk(&self) -> GroupVersionKind {
        GroupVersionKind::gvk(T::GROUP, T::VERSION, T::KIND)
    }
}

impl<T> PseudoGvkBuilderExt for T
where
    T: Metadata<Ty = ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
{
    fn build_pseudo_gvk(&self, extractor_name: &str) -> GroupVersionKind {
        let mut gvk = self.gvk();
        let parts = [&gvk.kind, extractor_name, &self.name_any()];
        gvk.kind = parts.join(PSEUDO_RESOURCE_JOIN_SEQ);
        gvk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_openapi::api::core::v1::Pod;

    #[test]
    fn test_pseudo_trait() {
        let mut gvk = Pod::gvk_for_type();
        assert!(gvk.get_pseudo_parent().is_none());

        gvk.kind = "Pod/test/1".to_string();
        assert!(gvk.get_pseudo_parent().is_some());
        assert_eq!(gvk.get_pseudo_parent().unwrap().kind, "Pod/test");
    }
}
