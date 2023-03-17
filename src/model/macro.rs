#[macro_export]
macro_rules! mk_resource_enum {
    (
        $name:ident,
            $(
                $opt_name:ident
            ),+
    ) => {

        #[derive(Debug, Clone, strum_macros::EnumIter, strum_macros::EnumString, strum_macros::IntoStaticStr)]
        pub enum $name {
            $(
                $opt_name(std::sync::Arc<k8s_openapi::api::core::v1::$opt_name>),
            )+

            DynamicObject(std::sync::Arc<$crate::model::dynamic_object::DynamicObjectWrapper>),
            PseudoResouce(std::sync::Arc<$crate::model::pseudo_resource::PseudoResource>),
        }

        // uid()
        impl $name {
            pub fn uid(&self) -> Option<String> {
                match self {
                    $(
                        Self::$opt_name(r) => r.uid(),
                    )+
                    Self::DynamicObject(r) => r.uid(),
                    Self::PseudoResouce(r) => r.uid(),
                }
            }
        }

        // name()
        impl $name {
            pub fn name(&self) -> String {
                match self {
                    $(
                        Self::$opt_name(r) => r.name_any(),
                    )+
                    Self::DynamicObject(r) => r.name_any(),
                    Self::PseudoResouce(r) => r.name(),
                }
            }
        }

        // namespace()
        impl $name {
            pub fn namespace(&self) -> String {
                match self {
                    $(
                        Self::$opt_name(r) => r.namespace().unwrap_or_default(),
                    )+
                    Self::DynamicObject(r) => r.namespace().unwrap_or_default(),
                    Self::PseudoResouce(r) => r.namespace(),
                }
            }
        }

        // age()
        impl $name {
            pub fn age(&self) -> chrono::Duration {
                match self {
                    $(
                        Self::$opt_name(r) => extract_age!(r),
                    )+
                    Self::DynamicObject(r) => extract_age!(r),
                    Self::PseudoResouce(r) => r.age(),
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
                    Self::PseudoResouce(r) => r.to_yaml(),
                }
            }

            fn to_json(&self) -> Result<String, k8s_openapi::serde_json::Error> {
                match self {
                    $(
                        Self::$opt_name(r) => k8s_openapi::serde_json::to_string(r.as_ref()),
                    )+
                    Self::DynamicObject(r) => r.to_json(),
                    Self::PseudoResouce(r) => r.to_json(),
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

        $(
            impl $crate::model::traits::MarkerTraitForStaticCases for k8s_openapi::api::core::v1::$opt_name {}
        )+


        pub async fn reqister_any_gvk(registry: &mut $crate::backend::reflector_registry::ReflectorRegistry, gvk: kube::api::GroupVersionKind) {
            use k8s_openapi::Resource;
            use $crate::util::panics::ResultExt;
            match gvk {
                $(

                    kube::api::GroupVersionKind {
                        ref group,
                        ref version,
                        ref kind,
                    } if group == k8s_openapi::api::core::v1::$opt_name::GROUP &&
                        version == k8s_openapi::api::core::v1::$opt_name::VERSION &&
                        kind == k8s_openapi::api::core::v1::$opt_name::KIND => {
                        registry.register::<k8s_openapi::api::core::v1::$opt_name>().await;
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
                    Self::PseudoResouce(r) => r.gvk(),
                }
            }
        }
    }
}

#[macro_export]
macro_rules! extract_status {
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
