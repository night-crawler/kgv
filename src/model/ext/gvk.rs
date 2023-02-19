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
