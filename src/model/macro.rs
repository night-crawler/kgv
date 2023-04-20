#[macro_export]
macro_rules! mk_resource_enum {
    (
        $name:ident,
            $(
                $opt_name:ident
            ),+
    ) => {

        #[derive(Debug, Clone, strum_macros::IntoStaticStr)]
        pub(crate) enum $name {
            $(
                $opt_name(std::sync::Arc<$crate::reexports::k8s::$opt_name>),
            )+

            DynamicObject(std::sync::Arc<$crate::model::dynamic_object::DynamicObjectWrapper>),
            PseudoResource(std::sync::Arc<$crate::model::pseudo_resource::PseudoResource>),
        }

        // resource_version
        impl $name {
            pub(crate) fn resource_version(&self) -> Option<String> {
                use k8s_openapi::Metadata;
                match self {
                    $(
                        Self::$opt_name(r) => r.metadata().resource_version.clone(),
                    )+
                    Self::DynamicObject(r) => r.resource_version(),
                    Self::PseudoResource(r) => r.resource_version(),
                }
            }
        }

        // deletion_timestamp
        impl $name {
            pub(crate) fn deletion_timestamp(&self) -> Option<&chrono::DateTime<chrono::Utc>> {
                use k8s_openapi::Metadata;
                match self {
                    $(
                        Self::$opt_name(r) => Some(&r.metadata().deletion_timestamp.as_ref()?.0),
                    )+
                    Self::DynamicObject(r) => Some(&r.meta().deletion_timestamp.as_ref()?.0),
                    Self::PseudoResource(r) => r.deletion_timestamp(),
                }
            }
        }

        // uid()
        impl $name {
            pub(crate) fn uid(&self) -> Option<String> {
                match self {
                    $(
                        Self::$opt_name(r) => r.uid(),
                    )+
                    Self::DynamicObject(r) => r.uid(),
                    Self::PseudoResource(r) => r.uid(),
                }
            }
        }

        // name()
        impl $name {
            pub(crate) fn name(&self) -> String {
                match self {
                    $(
                        Self::$opt_name(r) => r.name_any(),
                    )+
                    Self::DynamicObject(r) => r.name_any(),
                    Self::PseudoResource(r) => r.name(),
                }
            }
        }

        // namespace()
        impl $name {
            pub(crate) fn namespace(&self) -> String {
                match self {
                    $(
                        Self::$opt_name(r) => r.namespace().unwrap_or_default(),
                    )+
                    Self::DynamicObject(r) => r.namespace().unwrap_or_default(),
                    Self::PseudoResource(r) => r.namespace(),
                }
            }
        }

        // age()
        impl $name {
            pub(crate) fn creation_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
                match self {
                    $(
                        Self::$opt_name(r) => r.creation_timestamp().unwrap_or_else(|| {
                            k8s_openapi::apimachinery::pkg::apis::meta::v1::Time(chrono::Utc::now())
                        }).0,
                    )+
                    Self::DynamicObject(r) => r.creation_timestamp().unwrap_or_else(|| {
                        k8s_openapi::apimachinery::pkg::apis::meta::v1::Time(chrono::Utc::now())
                    }).0,
                    Self::PseudoResource(r) => r.creation_timestamp()
                }
            }
        }

        // serialize to yaml ()
        impl $crate::model::traits::SerializeExt for $name {
            fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
                match self {
                    $(
                        Self::$opt_name(r) => serde_yaml::to_string(r.as_ref()),
                    )+
                    Self::DynamicObject(r) => r.to_yaml(),
                    Self::PseudoResource(r) => r.to_yaml(),
                }
            }

            fn to_json(&self) -> Result<String, k8s_openapi::serde_json::Error> {
                match self {
                    $(
                        Self::$opt_name(r) => k8s_openapi::serde_json::to_string(r.as_ref()),
                    )+
                    Self::DynamicObject(r) => r.to_json(),
                    Self::PseudoResource(r) => r.to_json(),
                }
            }
        }

        $(
            impl From<Arc<$opt_name>> for $name {
                fn from(resource: Arc<$opt_name>) -> Self {
                    ResourceView::$opt_name(resource)
                }
            }
        )+

        pub(crate) async fn register_any_gvk(registry: &mut $crate::backend::reflector_registry::ReflectorRegistry, gvk: kube::api::GroupVersionKind) {
            use k8s_openapi::Resource;
            use $crate::util::panics::ResultExt;
            match gvk {
                $(

                    kube::api::GroupVersionKind {
                        ref group,
                        ref version,
                        ref kind,
                    } if group == $crate::reexports::k8s::$opt_name::GROUP &&
                        version == $crate::reexports::k8s::$opt_name::VERSION &&
                        kind == $crate::reexports::k8s::$opt_name::KIND => {
                        registry.register::<$crate::reexports::k8s::$opt_name>().await;
                    }

                )+

                gvk => {
                    registry.register_gvk(gvk).await.unwrap_or_log();
                }
            }
        }

        impl $crate::traits::ext::gvk::GvkExt for $name {
            fn gvk(&self) -> kube::api::GroupVersionKind {
                match self {
                    $(
                        Self::$opt_name(resource) => resource.gvk(),
                    )+

                    Self::DynamicObject(wrapper) => wrapper.gvk(),
                    Self::PseudoResource(r) => r.gvk(),
                }
            }
        }
    }
}

#[macro_export]
macro_rules! extract_phase {
    ($val:expr) => {
        $val.status
            .as_ref()
            .and_then(|status| status.phase.as_ref())
            .cloned()
            .unwrap_or_default()
    };
}

#[macro_export]
macro_rules! extract_age {
    ($val:expr) => {
        $val.creation_timestamp()
            .map(|t| Utc::now() - t.0)
            .unwrap_or_else(|| chrono::Duration::seconds(0))
    };
}
