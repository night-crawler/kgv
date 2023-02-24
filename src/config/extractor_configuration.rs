use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use cursive::reexports::log::info;
use kube::api::GroupVersionKind;
use rhai::{Engine, AST};
use serde::{Deserialize, Serialize};

use crate::model::ext::gvk::GvkNameExt;
use crate::util::error::KgvError;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ResourceExtractorConfigProps {
    resources: Vec<ResourceConfigProps>,
}

impl TryFrom<PathBuf> for ResourceExtractorConfigProps {
    type Error = KgvError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let f = File::open(value)?;
        Ok(serde_yaml::from_reader(f)?)
    }
}

impl TryFrom<&str> for ResourceExtractorConfigProps {
    type Error = KgvError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(serde_yaml::from_str(value)?)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum EmbeddedExtractor {
    Namespace,
    Name,
    Age,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
enum EvalConfigProps {
    ScriptPath { path: String },
    ScriptContent { content: String },
    Embedded { name: EmbeddedExtractor },
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct ResourceConfigProps {
    resource: GroupVersionKind,
    columns: Vec<ColumnConfigProps>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
struct ColumnConfigProps {
    name: String,
    display_name: String,
    width: usize,
    evaluator: EvalConfigProps,
}

#[derive(Debug)]
pub enum Evaluator {
    AST(AST),
    Embedded(EmbeddedExtractor),
}

#[derive(Debug)]
pub struct Column {
    pub name: String,
    pub display_name: String,
    pub width: usize,
    pub evaluator: Evaluator,
}

impl Column {
    fn from_config(config_column: ColumnConfigProps, engine: &Engine) -> anyhow::Result<Self> {
        let evaluator = match config_column.evaluator {
            EvalConfigProps::ScriptPath { path } => {
                let ast = engine.compile_file(PathBuf::from(path))?;
                Evaluator::AST(ast)
            }
            EvalConfigProps::ScriptContent { content } => {
                let ast = engine.compile(content)?;
                Evaluator::AST(ast)
            }
            EvalConfigProps::Embedded { name } => Evaluator::Embedded(name),
        };

        Ok(Self {
            name: config_column.name,
            display_name: config_column.display_name,
            width: config_column.width,
            evaluator,
        })
    }
}

pub struct ExtractionConfig {
    pub gvk_to_columns: HashMap<GroupVersionKind, Vec<Column>>,
}

impl TryFrom<ResourceExtractorConfigProps> for ExtractionConfig {
    type Error = KgvError;

    fn try_from(value: ResourceExtractorConfigProps) -> Result<Self, Self::Error> {
        let engine = Engine::new();
        let mut map: HashMap<GroupVersionKind, Vec<Column>> = HashMap::new();

        let now = std::time::Instant::now();
        for resource in value.resources {
            let mut columns: Vec<Column> = vec![];
            for config_column in resource.columns {
                let col_clone = config_column.clone();
                let column = match Column::from_config(config_column, &engine) {
                    Ok(col) => col,
                    Err(err) => {
                        return Err(KgvError::ContentCompileError(
                            resource.resource.full_name(),
                            col_clone.name,
                            format!("{:?}", col_clone.evaluator),
                            err,
                        ));
                    }
                };
                columns.push(column);
            }

            let gvk = resource.resource.clone();
            if map.insert(resource.resource, columns).is_some() {
                return Err(KgvError::DuplicateGvkError(gvk.full_name()));
            }
        }

        let num_columns: usize = map.values().map(|columns| columns.len()).sum();
        info!(
            "Imported {} GVKs with {} columns in {}",
            map.len(),
            num_columns,
            now.elapsed().as_millis()
        );

        Ok(Self {
            gvk_to_columns: map,
        })
    }
}

pub fn load_embedded_config() -> Result<ExtractionConfig, KgvError> {
    let resources = [include_str!("../../extractor_config_files/pod.yaml")];

    let mut map: HashMap<GroupVersionKind, Vec<Column>> = HashMap::new();

    for resource in resources {
        let config_props = ResourceExtractorConfigProps::try_from(resource)?;
        let config = ExtractionConfig::try_from(config_props)?;
        for (gvk, columns) in config.gvk_to_columns.into_iter() {
            if map.insert(gvk.clone(), columns).is_some() {
                return Err(KgvError::DuplicateGvkError(gvk.full_name()));
            }
        }
    }

    Ok(ExtractionConfig {
        gvk_to_columns: map,
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn test() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "resource").unwrap();

        let config = ResourceExtractorConfigProps {
            resources: vec![ResourceConfigProps {
                resource: GroupVersionKind::gvk("a", "b", "c"),
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
                        evaluator: EvalConfigProps::ScriptPath {
                            path: tmpfile.path().to_str().unwrap().to_string(),
                        },
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
            }],
        };

        let serialized = serde_yaml::to_string(&config).unwrap();
        let deserialized_config: ResourceExtractorConfigProps =
            serde_yaml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized_config);

        println!("{}", serialized);

        assert!(ExtractionConfig::try_from(deserialized_config).is_ok())
    }
}
