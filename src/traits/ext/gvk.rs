use itertools::Itertools;
use kube::api::GroupVersionKind;

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

pub trait GvkUiExt {
    fn list_view_table_name(&self) -> String;
    fn list_view_panel_name(&self) -> String;
    fn namespace_edit_view_name(&self) -> String;
    fn name_edit_view_name(&self) -> String;
}

impl GvkUiExt for GroupVersionKind {
    fn list_view_table_name(&self) -> String {
        format!("{}-table", self.full_name())
    }
    fn list_view_panel_name(&self) -> String {
        format!("{}-panel", self.full_name())
    }

    fn namespace_edit_view_name(&self) -> String {
        format!("{}-namespace-edit-view", self.full_name())
    }

    fn name_edit_view_name(&self) -> String {
        format!("{}-name-edit-view", self.full_name())
    }
}
