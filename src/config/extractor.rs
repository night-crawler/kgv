use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cursive::reexports::log::{error, info, warn};
use kube::api::GroupVersionKind;
use rhai::{Engine, AST};
use serde::{Deserialize, Serialize};

use crate::traits::ext::engine::EngineExt;
use crate::traits::ext::gvk::GvkNameExt;
use crate::util::error::KgvError;
use crate::util::fs::scan_files;
use crate::util::paths::resolve_path;
use crate::util::ui::ago;

#[derive(Debug, Default)]
pub struct ExtractorConfig {
    pub columns_map: HashMap<GroupVersionKind, Arc<Vec<Column>>>,
    pub templates_map: HashMap<GroupVersionKind, Arc<DetailsTemplate>>,
    pub pseudo_resources_map: HashMap<GroupVersionKind, Arc<Vec<PseudoResourceConf>>>,
}

impl ExtractorConfig {
    pub fn new(roots: &[PathBuf]) -> Self {
        let mut instance = Self::default();
        let now = std::time::Instant::now();
        let parsed_resources = parse_resource_dirs(roots);
        let elapsed = chrono::Duration::from_std(now.elapsed())
            .unwrap_or_else(|_| chrono::Duration::seconds(0));

        info!(
            "Parsed {} resources in {}",
            parsed_resources.len(),
            ago(elapsed)
        );

        let engine = Engine::new();

        let now = std::time::Instant::now();

        for (path, mut resource_config_props) in parsed_resources {
            let detail_config = resource_config_props.details.take();
            let columns = parse_resource_columns(&engine, &path, &resource_config_props);
            let pseudo_resources = parse_pseudo_resources(&engine, &path, &resource_config_props);
            let gvk = resource_config_props.resource.clone();

            if let Some(details) = detail_config {
                let (template_path, template) = parse_detail_templates(&path, details);
                instance.register_detail_template(gvk.clone(), template, &template_path);
            }

            instance.register_gvk_columns(gvk.clone(), columns, &path);
            instance.register_gvk_pseudo_resource_extractors(gvk.clone(), pseudo_resources, &path);
        }

        let elapsed = chrono::Duration::from_std(now.elapsed())
            .unwrap_or_else(|_| chrono::Duration::seconds(0));
        let num_columns: usize = instance
            .columns_map
            .values()
            .map(|columns| columns.len())
            .sum();

        info!(
            "Imported {} GVKs with {} columns in {}",
            instance.columns_map.len(),
            num_columns,
            ago(elapsed)
        );

        instance
    }

    fn register_gvk_columns(&mut self, gvk: GroupVersionKind, columns: Vec<Column>, origin: &Path) {
        let gvk_full_name = gvk.full_name();
        if self.columns_map.insert(gvk, columns.into()).is_some() {
            warn!(
                "{}: Replaced columns from {}",
                gvk_full_name,
                origin.display()
            );
        } else {
            info!(
                "{}: Loaded columns from {}",
                gvk_full_name,
                origin.display()
            );
        }
    }

    fn register_gvk_pseudo_resource_extractors(
        &mut self,
        gvk: GroupVersionKind,
        pseudo_resources: Vec<PseudoResourceConf>,
        origin: &Path,
    ) {
        let gvk_full_name = gvk.full_name();
        if self
            .pseudo_resources_map
            .insert(gvk, pseudo_resources.into())
            .is_some()
        {
            warn!(
                "{}: Replaced pseudo resources from {}",
                gvk_full_name,
                origin.display()
            );
        } else {
            info!(
                "{}: Imported pseudo resources from {}",
                gvk_full_name,
                origin.display()
            );
        }
    }

    fn register_detail_template(
        &mut self,
        gvk: GroupVersionKind,
        template: DetailsTemplate,
        origin: &Path,
    ) {
        let gvk_full_name = gvk.full_name();
        if self.templates_map.insert(gvk, template.into()).is_some() {
            warn!(
                "{}: Replaced detail template from {}",
                gvk_full_name,
                origin.display()
            );
        } else {
            info!(
                "{}: Loaded detail template from {}",
                gvk_full_name,
                origin.display()
            );
        }
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
pub struct HbsHelper {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct DetailsTemplateConfigProps {
    template: PathBuf,

    #[serde(default)]
    helpers: Vec<HbsHelper>,
}

#[derive(Debug)]
pub struct DetailsTemplate {
    pub template: PathBuf,
    pub helpers: Vec<HbsHelper>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct PseudoResourceExtractorConfigPros {
    name: String,
    script_content: String,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
struct ResourceConfigProps {
    resource: GroupVersionKind,
    #[serde(default)]
    imports: Vec<String>,

    #[serde(default)]
    pseudo_resources: Vec<PseudoResourceExtractorConfigPros>,

    details: Option<DetailsTemplateConfigProps>,
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
pub struct PseudoResourceConf {
    pub name: String,
    pub ast: AST,
}

impl PseudoResourceConf {
    fn try_from_config(
        config: &PseudoResourceExtractorConfigPros,
        engine: &Engine,
        imports: &[String],
    ) -> anyhow::Result<Self> {
        Ok(Self {
            name: config.name.clone(),
            ast: engine.compile_content_with_imports(&config.script_content, imports)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum EvaluatorType {
    AST(AST),
    Embedded(EmbeddedExtractor),
}

impl EvaluatorType {
    fn try_from_config(
        config: &EvalConfigProps,
        engine: &Engine,
        source_path: &Path,
        imports: &[String],
    ) -> anyhow::Result<Self> {
        let evaluator_type = match config {
            EvalConfigProps::ScriptPath { path: script_path } => {
                let script_path = resolve_path(source_path, script_path);
                EvaluatorType::AST(engine.compile_file_with_imports(&script_path, imports)?)
            }
            EvalConfigProps::ScriptContent { content } => {
                EvaluatorType::AST(engine.compile_content_with_imports(content, imports)?)
            }
            EvalConfigProps::Embedded { name } => EvaluatorType::Embedded(name.clone()),
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

fn parse_resource_dirs(roots: &[PathBuf]) -> Vec<(PathBuf, ResourceConfigProps)> {
    let mut parsed_props = vec![];
    for file in scan_files(roots) {
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

    parsed_props
}

fn parse_resource_columns(
    engine: &Engine,
    source_path: &Path,
    resource_config_props: &ResourceConfigProps,
) -> Vec<Column> {
    let mut columns: Vec<Column> = vec![];
    for column_config in &resource_config_props.columns {
        let column_name = column_config.name.clone();

        let evaluator_type = EvaluatorType::try_from_config(
            &column_config.evaluator,
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
            display_name: column_config.display_name.clone(),
            width: column_config.width,
            evaluator_type,
        };

        columns.push(column);
    }

    columns
}

fn parse_pseudo_resources(
    engine: &Engine,
    source_path: &Path,
    resource_config_props: &ResourceConfigProps,
) -> Vec<PseudoResourceConf> {
    let mut pseudo_resources: Vec<PseudoResourceConf> = vec![];
    for pseudo_resource_config in &resource_config_props.pseudo_resources {
        let pseudo_resource_name = pseudo_resource_config.name.clone();

        let pseudo_resource = PseudoResourceConf::try_from_config(
            pseudo_resource_config,
            engine,
            &resource_config_props.imports,
        );

        let pseudo_resource = match pseudo_resource {
            Ok(pseudo_resource) => pseudo_resource,
            Err(err) => {
                error!(
                    "Failed to process pseudo resource {} in file {}: {err}",
                    pseudo_resource_name,
                    source_path.display()
                );
                continue;
            }
        };

        pseudo_resources.push(pseudo_resource);
    }

    pseudo_resources
}

fn parse_detail_templates(
    path: &Path,
    details: DetailsTemplateConfigProps,
) -> (PathBuf, DetailsTemplate) {
    let template_path = resolve_path(path, &details.template);
    let template = DetailsTemplate {
        template: template_path.clone(),
        helpers: details
            .helpers
            .into_iter()
            .map(|mut helper| {
                helper.path = resolve_path(&template_path, &helper.path);
                helper
            })
            .collect(),
    };
    (template_path, template)
}

#[cfg(test)]
mod tests {
    use k8s_openapi::api::core::v1::Pod;

    use crate::traits::ext::gvk::GvkStaticExt;

    use super::*;

    #[test]
    fn test_empty() {
        assert!(ExtractorConfig::new(&[]).columns_map.is_empty());
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
            details: Some(DetailsTemplateConfigProps {
                template: Default::default(),
                helpers: vec![],
            }),
            pseudo_resources: Vec::new(),
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

        let data = serde_yaml::to_string(&resource).unwrap();
        println!("{}", data);

        std::fs::write(&extractor_path, data).unwrap();

        let deserialized = ExtractorConfig::new(&[extractor_dir.into_path()]);
        assert_eq!(deserialized.columns_map.len(), 1);
        assert_eq!(deserialized.columns_map.values().next().unwrap().len(), 3);
    }
}
