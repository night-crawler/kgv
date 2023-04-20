use std::path::PathBuf;

#[derive(Debug)]
pub(crate) struct KgvConfiguration {
    pub(crate) cache_dir: Option<PathBuf>,
    pub(crate) logs_dir: Option<PathBuf>,
    pub(crate) module_dirs: Vec<PathBuf>,
    pub(crate) extractor_dirs: Vec<PathBuf>,
    pub(crate) num_tokio_backend_threads: usize,
    pub(crate) num_dispatcher_threads: usize,
    pub(crate) num_evaluator_threads: usize,
    pub(crate) accept_invalid_certs: bool,
}
