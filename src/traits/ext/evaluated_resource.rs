use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};

pub trait EvaluatedResourceExt {
    fn is_pod(&self) -> bool;
}

impl EvaluatedResourceExt for EvaluatedResource {
    fn is_pod(&self) -> bool {
        match self.resource {
            ResourceView::Pod(_) => true,
            _ => false,
        }
    }
}
