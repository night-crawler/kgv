use std::sync::Arc;

use anyhow::Context;
use cursive::reexports::log::error;
use handlebars::Handlebars;
use rhai::Engine;

use crate::config::extractor::ExtractorConfig;
use crate::model::resource::resource_view::ResourceView;
use crate::model::traits::SerializeExt;
use crate::traits::ext::gvk::{GvkExt, GvkNameExt};
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

    pub fn render_html(&self, resource: &ResourceView) -> anyhow::Result<String> {
        let gvk = resource.gvk();
        let gvk_full_name = gvk.full_name();

        let extractor_config = self.extractor_config_watcher.value();
        let details_template = extractor_config
            .template_map
            .get(&gvk)
            .context(format!(
                "A template for GVK {gvk_full_name} is not registered"
            ))?;

        let engine = self.engine_watcher.build();
        let json = resource.to_json()?;
        let var = engine.parse_json(json, true)?;

        let mut hbs = Handlebars::new();
        hbs.set_engine(engine);

        if let Err(err) = hbs.register_template_file(&gvk_full_name, &details_template.template) {
            error!(
                "Failed to import a template for {gvk_full_name} at {}: {}",
                &details_template.template.display(),
                err
            );
        }

        for helper in &details_template.helpers {
            if let Err(err)=  hbs.register_script_helper_file(&helper.name, &helper.path) {
                error!("Failed to register a hbs helper: {helper:?}: {err}");
            }
        }

        let html = hbs.render(&resource.gvk().full_name(), &var)?;
        Ok(html)
    }
}
