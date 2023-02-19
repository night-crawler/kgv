#[macro_export]
macro_rules! mk_resource_enum {
    (
        $name:ident,
        [
            $(
                $opt_name:ident: [ $($column:expr),+ ]
            ),+
        ]
    ) => {

        #[derive(Debug, Clone, strum_macros::EnumIter, strum_macros::EnumString, strum_macros::IntoStaticStr)]
        pub enum $name {
            $(
                $opt_name(std::sync::Arc<k8s_openapi::api::core::v1::$opt_name>),
            )+

            DynamicObject(std::sync::Arc<$crate::model::DynamicObjectWrapper>)
        }

        // Default get_columns()
        impl $name {
            pub fn get_columns(&self) -> &[$crate::model::resource::resource_column::ResourceColumn] {
                match self {
                    $(
                        Self::$opt_name(_) => &[
                            $($column),+
                        ],
                    )+

                    Self::DynamicObject(_) => &[
                        $crate::model::resource::resource_column::ResourceColumn::Namespace,
                        $crate::model::resource::resource_column::ResourceColumn::Name,
                    ],
                }
            }
        }

        // Build column map
        impl $name {
            pub fn build_gvk_to_columns_map() -> std::collections::HashMap<
                kube::api::GroupVersionKind,
                Vec<$crate::model::resource::resource_column::ResourceColumn>
            > {
                use $crate::model::traits::GvkStaticExt;
                let mut map = std::collections::HashMap::new();

                $(
                    let gvk = k8s_openapi::api::core::v1::$opt_name::gvk_for_type();
                    let result = map.insert(
                        gvk.clone(),
                        vec![
                            $($column),+
                        ],
                    );
                    assert!(result.is_none(), "Duplicate value: {:?}", gvk);
                )+

                map
            }
        }

        // uid()
        impl $name {
            pub fn uid(&self) -> Option<String> {
                match self {
                    $(
                        Self::$opt_name(r) => r.uid(),
                    )+
                    Self::DynamicObject(r) => r.uid(),
                }
            }

            pub fn full_unique_name(&self) -> String {
                use $crate::model::traits::GvkExt;
                let gvk = self.gvk();
                format!(
                    "{}/{}/{}::{}/{}",
                    gvk.group, gvk.version, gvk.kind,
                    self.namespace(), self.name()
                )
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


        pub async fn reqister_any_gvk(registry: &mut $crate::model::reflector_registry::ReflectorRegistry, gvk: kube::api::GroupVersionKind) {
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

        impl $crate::model::traits::GvkExt for $name {
            fn gvk(&self) -> kube::api::GroupVersionKind {
                match self {
                    $(
                        Self::$opt_name(resource) => resource.gvk(),
                    )+

                    Self::DynamicObject(wrapper) => wrapper.gvk()
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
