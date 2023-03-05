use std::collections::HashMap;
use std::sync::Arc;

use cursive::reexports::log::error;
use kube::api::GroupVersionKind;

use crate::config::extractor::ColumnHandle;
use crate::eval::eval_result::EvalResult;
use crate::eval::evaluator::Evaluator;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::traits::ext::gvk::GvkExt;
use crate::ui::column_registry::ColumnRegistry;

pub struct ResourceManager {
    resources_by_gvk: HashMap<GroupVersionKind, HashMap<String, EvaluatedResource>>,
    evaluator: Evaluator,
    column_registry: ColumnRegistry,
}

impl ResourceManager {
    pub fn new(evaluator: Evaluator, column_registry: ColumnRegistry) -> Self {
        Self {
            evaluator,
            column_registry,
            resources_by_gvk: HashMap::default(),
        }
    }
    pub fn replace(&mut self, resource: ResourceView) -> EvaluatedResource {
        let key = resource.uid().unwrap_or_else(|| {
            error!("Received a resource without uid: {:?}", resource);
            resource.full_unique_name()
        });
        let gvk = resource.gvk();
        let columns = self.column_registry.get_columns(&gvk);

        let evaluated_resource = match self.evaluator.evaluate(resource.clone(), &columns) {
            Ok(evaluated_resource) => evaluated_resource,
            Err(err) => {
                error!(
                    "Failed to evaluate resource {}: {}",
                    resource.full_unique_name(),
                    err
                );
                let values = vec![EvalResult::Error("?".to_string()); columns.len()];
                EvaluatedResource {
                    values: Arc::new(values),
                    resource,
                }
            }
        };

        self.resources_by_gvk
            .entry(gvk)
            .or_default()
            .insert(key, evaluated_resource.clone());

        evaluated_resource
    }

    pub fn replace_all(&mut self, resources: Vec<ResourceView>) {
        resources.into_iter().for_each(|resource| {
            self.replace(resource);
        });
    }

    pub fn get_column_handles(&self, gvk: &GroupVersionKind) -> Vec<ColumnHandle> {
        self.column_registry.get_column_handles(gvk)
    }

    pub fn get_resources_iter(
        &self,
        gvk: &GroupVersionKind,
    ) -> impl Iterator<Item = &EvaluatedResource> {
        self.resources_by_gvk
            .get(gvk)
            .map(|map| map.values())
            .into_iter()
            .flatten()
    }
}
