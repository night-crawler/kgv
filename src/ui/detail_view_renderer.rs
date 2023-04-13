use std::sync::Arc;

use handlebars::{
    Context, Helper, HelperResult, JsonRender, Output,
    RenderContext,
};
use handlebars::Handlebars;
use itertools::Itertools;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use rhai::Engine;

use crate::config::extractor::{DetailsTemplate, ExtractorConfig};
use crate::model::resource::resource_view::ResourceView;
use crate::model::traits::SerializeExt;
use crate::traits::ext::gvk::{GvkExt, GvkNameExt};
use crate::util::error::{LogError, LogErrorOptionExt, LogErrorResultExt};
use crate::util::ui::compute_age;
use crate::util::watcher::LazyWatcher;

pub struct DetailViewRenderer {
    engine_watcher: Arc<LazyWatcher<Engine>>,
    extractor_config_watcher: Arc<LazyWatcher<ExtractorConfig>>,
}

impl DetailViewRenderer {
    pub fn new(
        engine_watcher: &Arc<LazyWatcher<Engine>>,
        extractor_config_watcher: &Arc<LazyWatcher<ExtractorConfig>>,
    ) -> Self {
        Self {
            engine_watcher: Arc::clone(engine_watcher),
            extractor_config_watcher: Arc::clone(extractor_config_watcher),
        }
    }

    pub fn render_html(&self, resource: &ResourceView) -> Result<String, LogError> {
        let gvk = resource.gvk();
        let gvk_full_name = gvk.full_name();
        let extractor_config = self.extractor_config_watcher.value();

        let details_template = extractor_config
            .detail_templates_map
            .get(&gvk)
            .to_log_error(|| format!("A template for GVK {gvk_full_name} is not registered"))?;

        let json = resource.to_json().to_log_error(|err| {
            format!(
                "Failed serialize resource {} to json: {err}",
                resource.full_unique_name()
            )
        })?;
        let engine = self.engine_watcher.build();
        let var = engine.parse_json(json, true).to_log_error(|err| {
            format!(
                "Failed to convert a json into a rhai map for resource {}: {err}",
                resource.full_unique_name()
            )
        })?;

        let hbs = self.setup_hbs(engine, &resource.gvk().full_name(), details_template)?;

        let html = hbs
            .render(&resource.gvk().full_name(), &var)
            .to_log_error(|err| {
                format!(
                    "Failed to render html for resource {}: {err}",
                    resource.full_unique_name()
                )
            })?;
        Ok(html)
    }

    fn setup_hbs(
        &self,
        engine: Engine,
        gvk_full_name: &str,
        details_template: &DetailsTemplate,
    ) -> Result<Handlebars, LogError> {
        let mut hbs = build_handlebars();
        hbs.set_engine(engine);

        if let Some(parent) = details_template.template.parent() {
            hbs.register_templates_directory(".hbs", parent)
                .to_log_error(|err| {
                    format!(
                        "Failed to register templates directory {}: {err}",
                        parent.display()
                    )
                })?;
        }
        hbs.register_template_file(gvk_full_name, &details_template.template)
            .to_log_error(|err| {
                format!(
                    "Failed to import a template for {gvk_full_name} at {}: {}",
                    details_template.template.display(),
                    err
                )
            })?;

        for helper in &details_template.helpers {
            hbs.register_script_helper_file(&helper.name, &helper.path)
                .to_log_error(|err| {
                    format!("Failed to register a hbs helper: {helper:?}: {err}")
                })?;
        }

        Ok(hbs)
    }
}


fn build_handlebars<'reg>() -> Handlebars<'reg> {
    let mut hbs = Handlebars::new();

    fn pretty_any(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
        let param = h.param(0).unwrap();
        let value = param.value().render();
        let prettified = crate::eval::helpers::pretty_any(&value);
        out.write(&prettified)?;
        Ok(())
    }

    fn to_yaml(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
        let param = h.param(0).unwrap();
        if param.is_value_missing() {
            return Ok(());
        }
        let value = param.value();
        let serialized = serde_yaml::to_string(value).unwrap();
        out.write(&serialized)?;
        Ok(())
    }

    fn age(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
        let param = h.param(0).unwrap();
        let value = param.value().as_str().unwrap_or("");
        let age = compute_age(value);
        out.write(&age)?;
        Ok(())
    }

    fn urlencode(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
        let param = h.param(0).unwrap();
        let value = param.value().as_str().unwrap_or("error_wrong_argument");
        let result = percent_encode(value.as_bytes(), NON_ALPHANUMERIC).to_string();
        out.write(&result)?;
        Ok(())
    }

    fn join(h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output) -> HelperResult {
        let arr = h.param(0).unwrap();
        let delim = h.param(1).unwrap().value().as_str().unwrap();
        let arr = arr.value().as_array().map(|v| &v[..]).unwrap_or_default();

        let joined = arr.iter().filter_map(|item| item.as_str()).join(delim);
        out.write(&joined)?;
        Ok(())
    }

    hbs.register_helper("join", Box::new(join));
    hbs.register_helper("pretty_any", Box::new(pretty_any));
    hbs.register_helper("to_yaml", Box::new(to_yaml));
    hbs.register_helper("age", Box::new(age));
    hbs.register_helper("urlencode", Box::new(urlencode));

    hbs
}