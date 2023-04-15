use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use cursive::reexports::log::{error, info, warn};
use kube::api::GroupVersionKind;

use crate::config::extractor::{
    ActionType, Column, EmbeddedExtractor, EvaluatorType, EventHandlerType, ExtractorConfig,
};
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
    tombstones: HashMap<String, chrono::DateTime<Utc>>,
}

impl ResourceManager {
    pub fn new(evaluator: Evaluator, config_watcher: &Arc<LazyWatcher<ExtractorConfig>>) -> Self {
        Self {
            evaluator,
            tombstones: HashMap::default(),
            config_watcher: Arc::clone(config_watcher),
            resources_by_gvk: HashMap::default(),
            default_columns: vec![
                Column {
                    name: "namespace".to_string(),
                    display_name: "Namespace".to_string(),
                    width: 0,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Namespace),
                },
                Column {
                    name: "name".to_string(),
                    display_name: "Name".to_string(),
                    width: 0,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Name),
                },
                Column {
                    name: "status".to_string(),
                    display_name: "Status".to_string(),
                    width: 0,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Status),
                },
                Column {
                    name: "age".to_string(),
                    display_name: "Age".to_string(),
                    width: 4,
                    evaluator_type: EvaluatorType::Embedded(EmbeddedExtractor::Age),
                },
            ]
            .into(),
        }
    }

    pub fn replace(
        &mut self,
        resource: ResourceView,
    ) -> (EvaluatedResource, Vec<EvaluatedResource>) {
        let now = std::time::Instant::now();
        let (evaluated_resource, pseudo_resources) = self.replace_inner(resource);
        info!(
            "Resource {} was replaced in {}",
            evaluated_resource.resource.full_unique_name(),
            ago(chrono::Duration::from_std(now.elapsed()).unwrap())
        );
        (evaluated_resource, pseudo_resources)
    }

    fn has_race_condition(&self, resource: &ResourceView) -> Option<EvaluatedResource> {
        let gvk = resource.gvk();
        if let Some(map) = self.resources_by_gvk.get(&gvk) {
            let key = resource.uid_or_name();
            if let Some(evaluated_resource) = map.get(&key) {
                let existing_version = evaluated_resource.resource.resource_version();
                let new_version = resource.resource_version();
                if compare_resource_versions(&existing_version, &new_version) == Ordering::Greater {
                    error!("Race condition detected for resource {}; existing version: {:?}, new version: {:?}",
                        resource.full_unique_name(),
                        existing_version,
                        new_version
                    );
                    return Some(evaluated_resource.clone());
                }
            }
        }
        None
    }

    fn replace_inner(
        &mut self,
        resource: ResourceView,
    ) -> (EvaluatedResource, Vec<EvaluatedResource>) {
        if let Some(old_evaluated_resource) = self.has_race_condition(&resource) {
            return (old_evaluated_resource, vec![]);
        }

        let is_deleted = resource.deletion_timestamp().is_some();

        let pseudo_resources = self.extract_pseudo_resources(&resource);
        let key = resource.uid_or_name();
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

        if is_deleted {
            let iter = std::iter::once(&evaluated_resource).chain(pseudo_resources.iter());
            for resource in iter {
                let key = resource.resource.uid_or_name();
                self.tombstones.insert(key, Utc::now());
            }
        }

        (evaluated_resource, pseudo_resources)
    }

    pub fn reevaluate_all_for_gvk(&mut self, gvk: &GroupVersionKind) {
        if let Some(resource_map) = self.resources_by_gvk.remove(gvk) {
            for (_, resource) in resource_map.into_iter() {
                self.replace(resource.resource);
            }
        }
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

    fn extract_pseudo_resources(&mut self, resource: &ResourceView) -> Vec<EvaluatedResource> {
        let config = self.config_watcher.value();
        let extractors = if let Some(extractors) = config.pseudo_resources_map.get(&resource.gvk())
        {
            extractors
        } else {
            return vec![];
        };

        let pseudo_resources = self
            .evaluator
            .evaluate_pseudo_resources(resource, extractors.as_ref());

        if pseudo_resources.is_empty() {
            return vec![];
        }
        info!(
            "Extracted {} pseudo resources from {}",
            pseudo_resources.len(),
            resource.full_unique_name()
        );

        let mut result = vec![];
        for pseudo_resource in pseudo_resources {
            let (resource, pseudo_resources) =
                self.replace(ResourceView::PseudoResource(Arc::new(pseudo_resource)));
            result.push(resource);
            result.extend(pseudo_resources);
        }

        result
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
            .filter(|resource| {
                // user must be able to see pseudo resources even if they are deleted
                // (i.e. from the pod view when pod was deleted, containers should still be visible
                let is_pseudo = matches!(resource.resource, ResourceView::PseudoResource(_));
                if is_pseudo {
                    return true;
                }
                let key = resource.resource.uid_or_name();
                !self.tombstones.contains_key(&key)
            })
    }

    pub fn get_submit_handler_type(&self, gvk: &GroupVersionKind) -> Option<ActionType> {
        self.config_watcher
            .value()
            .event_handler_types_map
            .get(gvk)
            .cloned()
            .iter()
            .flat_map(|handlers| handlers.iter())
            .find_map(|event_handler_type| match event_handler_type {
                EventHandlerType::Submit { action } => Some(action.clone()),
                _ => None,
            })
    }

    pub fn get_resource_by_uid(&self, uid: &str) -> Option<EvaluatedResource> {
        for map in self.resources_by_gvk.values() {
            if let Some(resource) = map.get(uid) {
                return Some(resource.clone());
            }
        }
        None
    }
}

fn compare_resource_versions(left: &Option<String>, right: &Option<String>) -> Ordering {
    if left.is_none() || right.is_none() {
        return left.cmp(right);
    }

    let (left, right) = (left.as_ref().unwrap(), right.as_ref().unwrap());

    if let (Ok(left), Ok(right)) = (left.parse::<usize>(), right.parse::<usize>()) {
        return left.cmp(&right);
    }

    left.cmp(right)
}
