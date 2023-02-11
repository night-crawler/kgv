
#[macro_export]
macro_rules! mk_filter_enum {
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
        }

        impl $name {
            pub fn get_columns(&self) -> &[$crate::ui::resource_column::ResourceColumn] {
                match self {
                    $(
                        Self::$opt_name(_) => &[
                            $($column),+
                        ],
                    )+
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



        // lazy_static::lazy_static! {
        //     static ref $map: std::collections::BTreeMap<&'static str, &'static str> =
        //         $crate::parse::util::prepare_enum_map::<$name>();
        // }

        impl $crate::util::k8s::GvkExt for $name {
                fn gvk(&self) -> kube::api::GroupVersionKind {
                match self {
                    $(
                        Self::$opt_name(resource) => resource.gvk(),
                    )+
                }
            }
        }

        //  impl $crate::util::k8s::GvkExt for $name {
        //     fn gvk(&self) -> kube::api::GroupVersionKind {
        //         match self {
        //             $(
        //                 Self::$opt_name(resource) => (
        //                     &[
        //                         stringify!($opt_name),
        //                         $($column),+
        //                     ],
        //                     stringify!($opt_name)
        //                 ),
        //             )+
        //         }
        //     }
        //
        //     fn split_by_longest_alias(input: &str) -> Option<(&str, &str)> {
        //         $crate::parse::util::split_by_longest_alias(input, $map.iter().rev())
        //     }
        // }

        // impl std::fmt::Display for $name {
        //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //         match self {
        //             $(
        //                 Self::$opt_name => write!(f, "{}", stringify!($opt_name)),
        //             )+
        //         }
        //     }
        // }

    }
}

