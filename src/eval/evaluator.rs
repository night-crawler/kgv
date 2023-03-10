use std::ops::Deref;
use std::sync::Arc;

use cursive::reexports::log::error;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefIterator;
use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};
use rhai::{Dynamic, Engine, Scope};

use crate::config::extractor::{Column, EmbeddedExtractor, EvaluatorType};
use crate::eval::eval_result::EvalResult;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::util::error::KgvError;
use crate::util::panics::ResultExt;
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

    pub fn evaluate(
        &self,
        resource: ResourceView,
        columns: &[Column],
    ) -> Result<EvaluatedResource, KgvError> {
        let map: rhai::Map = self.pool.install(|| {
            let engine = self.watcher.value();

            let json = match resource.to_json() {
                Ok(json) => json,
                Err(err) => return Err(KgvError::SerdeJsonError(err)),
            };

            match engine.parse_json(&json, true) {
                Ok(parsed_json) => Ok(parsed_json),
                Err(err) => Err(KgvError::EngineJsonParseError(json, *err)),
            }
        })?;

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
                        if value.is_string() {
                            return EvalResult::String(value.into_string().unwrap_or_log());
                        } else if value.is_int() {
                            return EvalResult::Int(value.as_int().unwrap_or_log());
                        }

                        let type_name = value.type_name();
                        match value.try_cast::<EvalResult>() {
                            Some(eval_result) => eval_result,
                            None => {
                                let error_message = format!(
                                    "Returned value for column '{}' (code: {}) is not EvalResult: {}",
                                    column.name,
                                    ast.source().unwrap_or(""),
                                    type_name
                                );
                                error!("{}", error_message);
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
    use k8s_openapi::serde_json::json;

    use crate::eval::engine_factory::build_engine;

    use super::*;

    #[test]
    fn test() {
        let pod: Pod = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": { "name": "example" },
            "spec": {
                "containers": [{
                    "name": "example",
                    "image": "alpine",
                    "command": ["tail", "-f", "/dev/null"],
                }],
            }
        }))
        .unwrap();

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

        let result = evaluator.evaluate(resource, &columns);
        assert!(result.is_ok());
    }
}
