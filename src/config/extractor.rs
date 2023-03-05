use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::path::{Path, PathBuf};

use cursive::reexports::log::{error, info, warn};
use kube::api::GroupVersionKind;
use rhai::{Engine, AST};
use serde::{Deserialize, Serialize};

use crate::traits::ext::engine::EngineExt;
use crate::traits::ext::gvk::GvkNameExt;
use crate::util::error::KgvError;
use crate::util::ui::ago;

pub struct DeserializedResources {
    resources: Vec<(PathBuf, ResourceConfigProps)>,
}

impl DeserializedResources {
    pub fn new(roots: &[PathBuf]) -> Self {
        let now = std::time::Instant::now();
        let mut parsed_props = vec![];
        for file in get_files(roots) {
            if let Some(ext) = file.extension() {
                if ext != "yaml" && ext != "yml" {
                    continue;
                }
            } else {
                continue;
            }
            match ResourceConfigProps::try_from(&file) {
                Ok(resource_config_props) => {
                    parsed_props.push((file, resource_config_props));
                }
                Err(err) => {
                    error!(
                        "Failed to parse file {} into resource configuration properties: {}",
                        file.display(),
                        err
                    );
                }
            }
        }

        let elapsed = chrono::Duration::from_std(now.elapsed())
            .unwrap_or_else(|_| chrono::Duration::seconds(0));

        info!(
            "Parsed {} resources in {}",
            parsed_props.len(),
            ago(elapsed)
        );

        Self {
            resources: parsed_props,
        }
    }

    pub fn into_map(self) -> HashMap<GroupVersionKind, Vec<Column>> {
        let engine = Engine::new();
        let mut map: HashMap<GroupVersionKind, Vec<Column>> = HashMap::new();

        let now = std::time::Instant::now();

        for (path, resource_config_props) in self.resources {
            let (gvk, columns) = Self::process_resource(&engine, &path, resource_config_props);
            let gvk_full_name = gvk.full_name();
            if map.insert(gvk, columns).is_some() {
                warn!(
                    "Replaced GVK {} with a new one from {}",
                    gvk_full_name,
                    path.display()
                );
            }
        }

        let elapsed = chrono::Duration::from_std(now.elapsed())
            .unwrap_or_else(|_| chrono::Duration::seconds(0));
        let num_columns: usize = map.values().map(|columns| columns.len()).sum();

        info!(
            "Imported {} GVKs with {} columns in {}",
            map.len(),
            num_columns,
            ago(elapsed)
        );

        map
    }

    fn process_resource(
        engine: &Engine,
        source_path: &Path,
        resource_config_props: ResourceConfigProps,
    ) -> (GroupVersionKind, Vec<Column>) {
        let mut columns: Vec<Column> = vec![];
        for column_config in resource_config_props.columns {
            let column_name = column_config.name.clone();

            let evaluator_type = EvaluatorType::try_from_config(
                column_config.evaluator,
                engine,
                source_path,
                &resource_config_props.imports,
            );

            let evaluator_type = match evaluator_type {
                Ok(col) => col,
                Err(err) => {
                    error!(
                        "Failed to process column {} in file {}: {err}",
                        column_name,
                        source_path.display()
                    );
                    continue;
                }
            };

            let column = Column {
                name: column_name,
                display_name: column_config.display_name,
                width: column_config.width,
                evaluator_type,
            };

            columns.push(column);
        }

        (resource_config_props.resource, columns)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum EmbeddedExtractor {
    Namespace,
    Name,
    Status,
    Age,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
enum EvalConfigProps {
    ScriptPath { path: PathBuf },
    ScriptContent { content: String },
    Embedded { name: EmbeddedExtractor },
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct ResourceConfigProps {
    resource: GroupVersionKind,
    #[serde(default)]
    imports: Vec<String>,
    columns: Vec<ColumnConfigProps>,
}

impl TryFrom<&PathBuf> for ResourceConfigProps {
    type Error = KgvError;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(value)?;
        let config_props: Self = serde_yaml::from_reader(file)?;
        Ok(config_props)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
struct ColumnConfigProps {
    name: String,
    display_name: String,
    width: usize,
    evaluator: EvalConfigProps,
}

#[derive(Debug, Clone)]
pub enum EvaluatorType {
    AST(AST),
    Embedded(EmbeddedExtractor),
}

impl EvaluatorType {
    fn try_from_config(
        config: EvalConfigProps,
        engine: &Engine,
        source_path: &Path,
        imports: &[String],
    ) -> anyhow::Result<Self> {
        let evaluator_type = match config {
            EvalConfigProps::ScriptPath { path: script_path } => {
                let script_path = if script_path.is_absolute() {
                    script_path
                } else if let Some(parent) = source_path.parent() {
                    parent.join(script_path)
                } else {
                    script_path
                };

                EvaluatorType::AST(engine.compile_file_with_imports(&script_path, imports)?)
            }
            EvalConfigProps::ScriptContent { content } => {
                EvaluatorType::AST(engine.compile_content_with_imports(&content, imports)?)
            }
            EvalConfigProps::Embedded { name } => EvaluatorType::Embedded(name),
        };

        Ok(evaluator_type)
    }
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub display_name: String,
    pub width: usize,
    pub evaluator_type: EvaluatorType,
}

#[derive(Debug)]
pub struct ColumnHandle {
    pub name: String,
    pub display_name: String,
    pub width: usize,
}

impl From<&Column> for ColumnHandle {
    fn from(column: &Column) -> Self {
        Self {
            name: column.name.clone(),
            display_name: column.display_name.clone(),
            width: column.width,
        }
    }
}

pub fn get_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut queue = VecDeque::from_iter(roots.iter().cloned());
    let mut files = vec![];
    while let Some(path) = queue.pop_front() {
        if path.is_dir() {
            match std::fs::read_dir(&path) {
                Ok(dir) => {
                    for dir_entry in dir {
                        match dir_entry {
                            Ok(dir_entry) => {
                                let path = dir_entry.path();
                                if path.is_file() {
                                    files.push(path);
                                } else {
                                    queue.push_back(path);
                                }
                            }
                            Err(err) => {
                                error!("Failed to get dir entry: {err}");
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to read dir {}: {}", path.display(), err);
                }
            }
        } else {
            files.push(path.clone());
        }
    }

    files
}

#[derive(Debug)]
pub struct ExtractionConfig {
    pub gvk_to_columns: HashMap<GroupVersionKind, Vec<Column>>,
}

#[cfg(test)]
mod tests {
    use k8s_openapi::api::core::v1::Pod;

    use crate::traits::ext::gvk::GvkStaticExt;

    use super::*;

    #[test]
    fn test_empty() {
        assert!(DeserializedResources::new(&[]).into_map().is_empty());
    }

    #[test]
    fn test() {
        let script_dir = tempfile::tempdir().unwrap();
        let script_path = script_dir.path().join("pod.rhai");
        std::fs::write(&script_path, "resource").unwrap();

        let extractor_dir = tempfile::tempdir().unwrap();
        let extractor_path = extractor_dir.path().join("pod.yaml");

        let resource = ResourceConfigProps {
            resource: Pod::gvk_for_type(),
            imports: vec![r##"import "pod" as pod;"##.to_string()],
            columns: vec![
                ColumnConfigProps {
                    name: "name".to_string(),
                    display_name: "name".to_string(),
                    width: 0,
                    evaluator: EvalConfigProps::Embedded {
                        name: EmbeddedExtractor::Name,
                    },
                },
                ColumnConfigProps {
                    name: "sample".to_string(),
                    display_name: "sample".to_string(),
                    width: 0,
                    evaluator: EvalConfigProps::ScriptPath { path: script_path },
                },
                ColumnConfigProps {
                    name: "sample2".to_string(),
                    display_name: "sample2".to_string(),
                    width: 0,
                    evaluator: EvalConfigProps::ScriptContent {
                        content: "resource".to_string(),
                    },
                },
            ],
        };

        std::fs::write(&extractor_path, serde_yaml::to_string(&resource).unwrap()).unwrap();

        let deserialized = DeserializedResources::new(&[extractor_dir.into_path()]);
        let map = deserialized.into_map();
        assert_eq!(map.len(), 1);
        assert_eq!(map.values().next().unwrap().len(), 3);
    }
}
