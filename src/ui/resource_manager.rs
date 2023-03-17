use std::collections::HashMap;
use std::sync::Arc;

use cursive::reexports::log::{error, info, warn};
use kube::api::GroupVersionKind;

use crate::config::extractor::{Column, DetailType, EmbeddedExtractor, EvaluatorType, ExtractorConfig};
use crate::eval::eval_result::EvalResult;
use crate::eval::evaluator::Evaluator;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::gvk::{GvkExt, PseudoResourceGvkExt};
use crate::util::ui::ago;
use crate::util::watcher::LazyWatcher;

pub struct ResourceManager {
    resources_by_gvk: HashMap<GroupVersionKind, HashMap<String, EvaluatedResource>>,
    evaluator: Evaluator,
    config_watcher: Arc<LazyWatcher<ExtractorConfig>>,
    default_columns: Arc<Vec<Column>>,
}

impl ResourceManager {
    pub fn new(evaluator: Evaluator, config_watcher: &Arc<LazyWatcher<ExtractorConfig>>) -> Self {
        Self {
            evaluator,
            config_watcher: Arc::clone(config_watcher),
            resources_by_gvk: HashMap::default(),
            default_columns: vec![
                Column {
                    name: "namespace".to_string(),
                    display_name: "Namespace".to_string(),
                    width: 10,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Namespace),
                },
                Column {
                    name: "name".to_string(),
                    display_name: "Name".to_string(),
                    width: 10,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Name),
                },
                Column {
                    name: "status".to_string(),
                    display_name: "Status".to_string(),
                    width: 10,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Status),
                },
                Column {
                    name: "age".to_string(),
                    display_name: "Age".to_string(),
                    width: 10,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Age),
                },
            ]
            .into(),
        }
    }

    pub fn replace(&mut self, resource: ResourceView) -> EvaluatedResource {
        let now = std::time::Instant::now();
        let evaluated_resource = self.replace_inner(resource);
        info!(
            "Resource {} was replaced in {}",
            evaluated_resource.resource.full_unique_name(),
            ago(chrono::Duration::from_std(now.elapsed()).unwrap())
        );
        evaluated_resource
    }

    fn replace_inner(&mut self, resource: ResourceView) -> EvaluatedResource {
        self.extract_pseudo_resources(&resource);
        let key = resource.uid().unwrap_or_else(|| {
            error!("Received a resource without uid: {:?}", resource);
            resource.full_unique_name()
        });
        let gvk = resource.gvk();
        let columns = self.get_columns(&gvk);

        let evaluated_resource = match self
            .evaluator
            .evaluate_columns(resource.clone(), columns.as_ref())
        {
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

    pub fn get_columns(&self, gvk: &GroupVersionKind) -> Arc<Vec<Column>> {
        if let Some(columns) = self.config_watcher.value().columns_map.get(gvk) {
            return Arc::clone(columns);
        }

        let mut gvk = gvk.clone();
        while let Some(parent_gvk) = gvk.get_pseudo_parent() {
            if let Some(columns) = self.config_watcher.value().columns_map.get(&parent_gvk) {
                return Arc::clone(columns);
            }
            gvk = parent_gvk;
        }

        warn!("Columns for GVK {:?} were not found", gvk.full_name());
        Arc::clone(&self.default_columns)
    }

    fn extract_pseudo_resources(&mut self, resource: &ResourceView) {
        if let Some(extractors) = self
            .config_watcher
            .value()
            .pseudo_resources_map
            .get(&resource.gvk())
        {
            let pseudo_resources = self
                .evaluator
                .evaluate_pseudo_resources(resource, extractors.as_ref());
            if pseudo_resources.is_empty() {
                return;
            }
            info!(
                "Extracted {} pseudo resources from {}",
                pseudo_resources.len(),
                resource.full_unique_name()
            );
            for pseudo_resource in pseudo_resources {
                self.replace(ResourceView::PseudoResouce(Arc::new(pseudo_resource)));
            }
        }
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
    
    pub fn get_detail_type(&self, gvk: &GroupVersionKind) -> Option<Arc<DetailType>> {
        self.config_watcher.value().detail_types_map.get(gvk).cloned()
    }
}
