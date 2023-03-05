use std::sync::Arc;

use cursive::reexports::log::info;
use kube::api::GroupVersionKind;

use crate::config::extractor::{
    Column, ColumnHandle, EmbeddedExtractor, EvaluatorType, ExtractorConfig,
};
use crate::util::watcher::LazyWatcher;

pub struct ColumnRegistry {
    watcher: Arc<LazyWatcher<ExtractorConfig>>,
    default_columns: Vec<Column>,
}

impl ColumnRegistry {
    pub fn new(watcher: &Arc<LazyWatcher<ExtractorConfig>>) -> Self {
        Self {
            watcher: Arc::clone(watcher),
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
            ],
        }
    }
    pub fn get_columns(&self, gvk: &GroupVersionKind) -> Vec<Column> {
        if let Some(columns) = self.watcher.value().columns_map.get(gvk) {
            columns.to_vec()
        } else {
            info!("Columns for GVK {:?} were not found", gvk);
            self.default_columns.to_vec()
        }
    }

    pub fn get_column_handles(&self, gvk: &GroupVersionKind) -> Vec<ColumnHandle> {
        if let Some(columns) = self.watcher.value().columns_map.get(gvk) {
            columns.iter().map(ColumnHandle::from).collect()
        } else {
            info!("Columns for GVK {:?} were not found", gvk);
            self.default_columns
                .iter()
                .map(ColumnHandle::from)
                .collect()
        }
    }
}
