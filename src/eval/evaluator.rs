use std::ops::Deref;
use std::sync::Arc;

use cursive::reexports::log::{error, info};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};
use rhai::{Dynamic, Engine, Scope};

use crate::config::extractor::{Column, EmbeddedExtractor, EvaluatorType, PseudoResourceConf};
use crate::eval::eval_result::{EvalResult, RhaiPseudoResource};
use crate::model::pseudo_resource::PseudoResource;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::util::error::KgvError;
use crate::util::watcher::LazyWatcher;

pub struct Evaluator {
    pool: ThreadPool,
    watcher: Arc<LazyWatcher<Engine>>,
}

impl Evaluator {
    pub fn new(
        num_threads: usize,
        watcher: &Arc<LazyWatcher<Engine>>,
    ) -> Result<Self, ThreadPoolBuildError> {
        let pool = ThreadPoolBuilder::new()
            .thread_name(|n| format!("eval-{n}"))
            .num_threads(num_threads)
            .build()?;

        Ok(Self {
            watcher: Arc::clone(watcher),
            pool,
        })
    }

    pub fn evaluate_pseudo_resources(
        &self,
        resource: ResourceView,
        extractors: &[PseudoResourceConf],
    ) -> Vec<PseudoResource> {
        let mut scope = Scope::new();
        scope.push("resource", self.to_rhai_object(&resource).unwrap());

        let pseudo_resources: Vec<(String, Vec<RhaiPseudoResource>)> = self.pool.install(|| {
            let engine = self.watcher.value();
            extractors
                .par_iter()
                .filter_map(|extractor| {
                    match Self::evaluate_pseudo_resource(
                        engine.as_ref(),
                        extractor,
                        scope.clone_visible(),
                    ) {
                        Ok(pseudo_resources) => {
                            info!("Evaluated pseudo resource: {:?}", pseudo_resources);
                            Some((extractor.name.to_string(), pseudo_resources))
                        }
                        Err(err) => {
                            error!("Failed to evaluate pseudo resource: {}", err);
                            None
                        }
                    }
                })
                .collect()
        });

        pseudo_resources
            .into_iter()
            .flat_map(|(extractor_name, resources)| {
                let extractor_name = std::iter::repeat(extractor_name);
                extractor_name.zip(resources)
            })
            .map(|(extractor_name, rhai_pseudo_resource)| PseudoResource {
                id: rhai_pseudo_resource.id,
                extractor_name,
                resource: rhai_pseudo_resource.resource,
                source: resource.clone(),
            })
            .collect()
    }

    fn to_rhai_object(&self, resource: &ResourceView) -> Result<rhai::Map, KgvError> {
        self.pool.install(|| {
            let engine = self.watcher.value();

            let json = match resource.to_json() {
                Ok(json) => json,
                Err(err) => return Err(KgvError::SerdeJsonError(err)),
            };

            match engine.parse_json(&json, true) {
                Ok(parsed_json) => Ok(parsed_json),
                Err(err) => Err(KgvError::EngineJsonParseError(json, *err)),
            }
        })
    }

    pub fn evaluate_columns(
        &self,
        resource: ResourceView,
        columns: &[Column],
    ) -> Result<EvaluatedResource, KgvError> {
        let map = self.to_rhai_object(&resource)?;

        let mut scope = Scope::new();
        scope.push("resource", map);

        let values = self.pool.install(|| {
            let engine = self.watcher.value();

            columns
                .par_iter()
                .map(|col| {
                    Self::evaluate_column(engine.deref(), col, &resource, scope.clone_visible())
                })
                .collect::<Vec<_>>()
        });

        Ok(EvaluatedResource {
            values: Arc::new(values),
            resource,
        })
    }

    fn evaluate_embedded(extractor: &EmbeddedExtractor, resource: &ResourceView) -> EvalResult {
        match extractor {
            EmbeddedExtractor::Namespace => EvalResult::String(resource.namespace()),
            EmbeddedExtractor::Name => EvalResult::String(resource.name()),
            EmbeddedExtractor::Status => EvalResult::String(resource.status()),
            EmbeddedExtractor::Age => EvalResult::Duration(resource.age()),
        }
    }

    fn evaluate_pseudo_resource(
        engine: &Engine,
        extractor: &PseudoResourceConf,
        mut scope: Scope,
    ) -> Result<Vec<RhaiPseudoResource>, KgvError> {
        let value: Dynamic = engine.eval_ast_with_scope(&mut scope, &extractor.ast)?;
        let array = value.into_typed_array::<RhaiPseudoResource>()?;
        Ok(array)
    }

    pub fn evaluate_column(
        engine: &Engine,
        column: &Column,
        resource: &ResourceView,
        mut scope: Scope,
    ) -> EvalResult {
        match &column.evaluator_type {
            EvaluatorType::Embedded(embedded) => Self::evaluate_embedded(embedded, resource),

            EvaluatorType::AST(ast) => {
                let dynamic_result: Result<Dynamic, _> =
                    engine.eval_ast_with_scope(&mut scope, ast);
                match dynamic_result {
                    Ok(value) => {
                        let type_name = value.type_name();

                        match EvalResult::try_from(value) {
                            Ok(value) => value,
                            Err(err) => {
                                let error_message = format!(
                                    "Error evaluating column {}, code: {}, type: {}: {}",
                                    column.name,
                                    ast.source().unwrap_or(""),
                                    type_name,
                                    err
                                );
                                error!("{error_message}");
                                EvalResult::Error(error_message)
                            }
                        }
                    }
                    Err(err) => {
                        error!(
                            "Failed to evaluate column {} (code: {}): {}",
                            column.name,
                            ast.source().unwrap_or(""),
                            err
                        );
                        EvalResult::Error(format!("{}", err))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use k8s_openapi::api::core::v1::Pod;
    use k8s_openapi::serde_json;
    use k8s_openapi::serde_json::{json, Value};

    use crate::eval::engine_factory::build_engine;

    use super::*;

    fn pod_json() -> Value {
        json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": { "name": "example" },
            "spec": {
                "containers": [
                    {
                        "name": "example1",
                        "image": "alpine1",
                        "command": ["tail", "-f", "/dev/null"],
                    },
                    {
                        "name": "example2",
                        "image": "alpine2",
                        "command": ["tail", "-f", "/dev/null"],
                    }
                ],
            }
        })
    }

    #[test]
    fn test() {
        let pod: Pod = serde_json::from_value(pod_json()).unwrap();

        let watcher = Arc::new(LazyWatcher::new(vec![], build_engine).unwrap());
        let evaluator = Evaluator::new(10, &watcher).unwrap();

        let engine = build_engine(&[]);

        let resource = ResourceView::Pod(Arc::new(pod));
        let columns = [
            Column {
                name: "a".to_string(),
                display_name: "a".to_string(),
                width: 0,
                evaluator_type: EvaluatorType::AST(
                    engine.compile(r#"resource.metadata.name"#).unwrap(),
                ),
            },
            Column {
                name: "b".to_string(),
                display_name: "b".to_string(),
                width: 0,
                evaluator_type: EvaluatorType::AST(
                    engine
                        .compile(r#"Result::String(resource.spec.containers[0].name)"#)
                        .unwrap(),
                ),
            },
        ];

        let result = evaluator.evaluate_columns(resource, &columns);
        assert!(result.is_ok());
        let result = result.unwrap();
        for eval_result in result.values.iter() {
            let is_error = matches!(eval_result, EvalResult::Error(_));
            assert!(!is_error, "eval_result has an error: {:?}", eval_result);
        }
    }

    #[test]
    fn test_extract_vec() {
        let pod: Pod = serde_json::from_value(pod_json()).unwrap();

        let watcher = Arc::new(LazyWatcher::new(vec![], build_engine).unwrap());
        let evaluator = Evaluator::new(10, &watcher).unwrap();

        let engine = build_engine(&[]);

        let resource = ResourceView::Pod(Arc::new(pod));

        let columns = [Column {
            name: "extract_containers".to_string(),
            display_name: "extract_containers".to_string(),
            width: 0,
            evaluator_type: EvaluatorType::AST(
                engine.compile(r#"resource.spec.containers"#).unwrap(),
            ),
        }];

        let result = evaluator.evaluate_columns(resource, &columns);
        assert!(result.is_ok());
        let result = result.unwrap();
        for eval_result in result.values.iter() {
            let is_error = matches!(eval_result, EvalResult::Error(_));
            assert!(!is_error, "eval_result has an error: {:?}", eval_result);
        }
    }

    #[test]
    fn test_extract_pseudo() {
        let pod: Pod = serde_json::from_value(pod_json()).unwrap();

        let watcher = Arc::new(LazyWatcher::new(vec![], build_engine).unwrap());
        let evaluator = Evaluator::new(10, &watcher).unwrap();

        let engine = build_engine(&[]);

        let resource = ResourceView::Pod(Arc::new(pod));

        let pseudo_col_extractor_configs = [PseudoResourceConf {
            name: "sample".to_string(),
            ast: engine
                .compile(
                    r#"
            fn extract_containers(resource) {
                let resources = [];
                for container in resource?.spec?.containers {
                    resources.push(PseudoResource(container.name, container))
                }
                resources
            }
            extract_containers(resource)
            "#,
                )
                .unwrap(),
        }];

        let pseudo_resources =
            evaluator.evaluate_pseudo_resources(resource, &pseudo_col_extractor_configs);
        assert_eq!(pseudo_resources.len(), 2);
    }
}
