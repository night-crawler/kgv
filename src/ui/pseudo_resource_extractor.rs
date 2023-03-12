use std::sync::Arc;

use crate::config::extractor::ExtractorConfig;
use crate::util::watcher::LazyWatcher;

pub struct PseudoResourceExtractor {
    config_watcher: Arc<LazyWatcher<ExtractorConfig>>,
}

impl PseudoResourceExtractor {
    pub fn new(watcher: &Arc<LazyWatcher<ExtractorConfig>>) -> Self {
        Self {
            config_watcher: Arc::clone(watcher),
        }
    }

}
