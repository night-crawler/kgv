use std::path::PathBuf;

#[derive(Debug)]
pub struct KgvConfiguration {
    pub home_dir: Option<PathBuf>,
    pub cache_dir: Option<PathBuf>,
    pub logs_dir: Option<PathBuf>,
    pub module_dirs: Vec<PathBuf>,
    pub extractor_dirs: Vec<PathBuf>,
    pub detail_template_dirs: Vec<PathBuf>,
    pub num_tokio_backend_threads: usize,
    pub num_dispatcher_threads: usize,
    pub num_evaluator_threads: usize,
    pub accept_invalid_certs: bool,
}
