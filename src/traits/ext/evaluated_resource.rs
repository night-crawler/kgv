use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};

pub(crate) trait EvaluatedResourceExt {
    fn is_pod(&self) -> bool;
}

impl EvaluatedResourceExt for EvaluatedResource {
    fn is_pod(&self) -> bool {
        matches!(self.resource, ResourceView::Pod(_))
    }
}
