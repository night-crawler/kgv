use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::path::PathBuf;

use cursive::reexports::log::info;
use kube::api::GroupVersionKind;
use rhai::{Engine, AST};
use serde::{Deserialize, Serialize};

use crate::eval::build_engine;
use crate::traits::ext::gvk::GvkNameExt;
use crate::util::error::KgvError;
use crate::util::panics::OptionExt;
use crate::util::ui::ago;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ResourceExtractorConfigProps {
    resources: Vec<ResourceConfigProps>,
    #[serde(default)]
    relative_scripts_dir: OsString,
}

impl TryFrom<PathBuf> for ResourceExtractorConfigProps {
    type Error = KgvError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(&value)?;
        let mut config_props: ResourceExtractorConfigProps = serde_yaml::from_reader(file)?;
        if config_props.relative_scripts_dir.is_empty() {
            config_props.relative_scripts_dir = value.parent().unwrap_or_log().as_os_str().to_os_string();
        }
        Ok(config_props)
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
    Status,
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
pub enum EvaluatorType {
    AST(AST),
    Embedded(EmbeddedExtractor),
}

#[derive(Debug)]
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

impl Column {
    fn from_config(
        relative_path: PathBuf,
        config_column: ColumnConfigProps,
        engine: &Engine,
    ) -> anyhow::Result<Self> {
        let evaluator = match config_column.evaluator {
            EvalConfigProps::ScriptPath { path } => {
                let path = relative_path.join(path);
                if !path.exists() {
                    File::create(&path).unwrap();
                }
                let ast: AST = engine.compile_file(path)?;
                EvaluatorType::AST(ast)
            }
            EvalConfigProps::ScriptContent { content } => {
                let mut ast: AST = engine.compile(&content)?;
                ast.set_source(content);
                EvaluatorType::AST(ast)
            }
            EvalConfigProps::Embedded { name } => EvaluatorType::Embedded(name),
        };

        Ok(Self {
            name: config_column.name,
            display_name: config_column.display_name,
            width: config_column.width,
            evaluator_type: evaluator,
        })
    }
}

#[derive(Debug)]
pub struct ExtractionConfig {
    pub gvk_to_columns: HashMap<GroupVersionKind, Vec<Column>>,
}

impl TryFrom<ResourceExtractorConfigProps> for ExtractionConfig {
    type Error = KgvError;

    fn try_from(value: ResourceExtractorConfigProps) -> Result<Self, Self::Error> {
        let engine = build_engine();
        let mut map: HashMap<GroupVersionKind, Vec<Column>> = HashMap::new();

        let now = std::time::Instant::now();
        let relative = PathBuf::from(value.relative_scripts_dir);
        for resource in value.resources {
            let mut columns: Vec<Column> = vec![];
            for config_column in resource.columns {
                let col_clone = config_column.clone();
                let column = match Column::from_config(relative.clone(), config_column, &engine) {
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
            ago(chrono::Duration::from_std(now.elapsed()).unwrap())
        );

        Ok(Self {
            gvk_to_columns: map,
        })
    }
}

pub fn load_columns_config(path: PathBuf) -> Result<ExtractionConfig, KgvError> {
    let config_props = ResourceExtractorConfigProps::try_from(path)?;
    let config = ExtractionConfig::try_from(config_props)?;

    Ok(config)
}

pub fn load_embedded_columns_config() -> Result<ExtractionConfig, KgvError> {
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

    use k8s_openapi::api::core::v1::Pod;

    use crate::traits::ext::gvk::GvkStaticExt;

    use super::*;

    #[test]
    fn test() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "resource").unwrap();

        let config = ResourceExtractorConfigProps {
            relative_scripts_dir: OsString::new(),
            resources: vec![ResourceConfigProps {
                resource: Pod::gvk_for_type(),
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
