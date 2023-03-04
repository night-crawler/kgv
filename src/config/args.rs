use std::path::{Path, PathBuf};

use clap::Parser;
use cursive::reexports::log::{error, info};

use crate::config::kgv_configuration::KgvConfiguration;
use crate::util::error::KgvError;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = r###"kgv
"###
)]
pub struct Args {
    /// A directory where kgv configs are located. If not exists and no other dirs are
    /// configured, the default column layout will be applied.
    #[arg(long, default_value_os_t = get_home_dir())]
    home_dir: PathBuf,

    /// A directory where kgv cache is located. If not specified, it will be set to
    /// <kgv-home-dir>/cache. If the directory can't be accessed, no cache will be used.,
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// A directory where kgv logs will be stored. If not specified, it will be set to
    /// [<kgv-home-dir>/logs]
    #[arg(long)]
    logs_dir: Option<PathBuf>,

    /// A directory where kgv rhai modules are located. If not specified, it will be set to
    /// [<kgv-home-dir>/modules].
    #[arg(long)]
    module_dirs: Option<Vec<PathBuf>>,

    /// A list of directories where kgv list view column definitions are described. If not specified,
    /// it will be set to <kgv-home-dir>/views/list.
    #[arg(long)]
    extractor_dirs: Option<Vec<PathBuf>>,

    /// A list of directories where kgv detail views are described. If not specified,
    /// it will be set to <kgv-home-dir>/views/detail.
    #[arg(long)]
    detail_templates_dirs: Option<Vec<PathBuf>>,

    /// Number of tokio worker threads used to communicate with k8s cluster.
    #[arg(long, default_value_t = 2)]
    num_backend_threads: usize,

    /// Number of rhai engine evaluator threads. Each engine instance created per thread.
    #[arg(long, default_value_t = 2)]
    num_evaluator_threads: usize,

    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(clap::Subcommand, Debug)]
enum Action {
    /// Generate a configuration for a Group Version Kind with defaults
    Generate {},
    /// Run extraction on a given entity using current configuration
    Extract {
        /// A path to a fixture containing the resource content in YAML format
        #[arg(long)]
        fixture: PathBuf,
    },
}

fn get_home_dir() -> PathBuf {
    home::home_dir()
        .map(|home| home.join(".kgv"))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn get_logs_dir(kgv_home_dir: &Path) -> PathBuf {
    kgv_home_dir.join("logs")
}

pub fn get_cache_dir(kgv_home_dir: &Path) -> PathBuf {
    kgv_home_dir.join("cache")
}

pub fn get_module_dirs(kgv_home_dir: &Path) -> Vec<PathBuf> {
    vec![kgv_home_dir.join("modules")]
}

pub fn get_col_def_dirs(kgv_home_dir: &Path) -> Vec<PathBuf> {
    vec![kgv_home_dir.join("views").join("list")]
}

pub fn get_detail_templates_dirs(kgv_home_dir: &Path) -> Vec<PathBuf> {
    vec![kgv_home_dir.join("views").join("detail")]
}

impl TryFrom<Args> for KgvConfiguration {
    type Error = KgvError;

    fn try_from(value: Args) -> Result<Self, Self::Error> {
        let home_dir = value.home_dir;
        let cache_dir = value.cache_dir.unwrap_or_else(|| get_cache_dir(&home_dir));
        let logs_dir = value.logs_dir.unwrap_or_else(|| get_logs_dir(&home_dir));
        let module_dirs = value
            .module_dirs
            .unwrap_or_else(|| get_module_dirs(&home_dir));
        let extractor_dirs = value
            .extractor_dirs
            .unwrap_or_else(|| get_col_def_dirs(&home_dir));
        let detail_templates_dirs = value
            .detail_templates_dirs
            .unwrap_or_else(|| get_detail_templates_dirs(&home_dir));

        let home_dir = wrap_opt(home_dir, "home dir");
        let logs_dir = wrap_opt(logs_dir, "logs dir");
        let cache_dir = wrap_opt(cache_dir, "cache dir");

        let module_dirs = create_dirs(module_dirs, "module dirs");
        let extractor_dirs = create_dirs(extractor_dirs, "extractor dirs");
        let detail_templates_dirs = create_dirs(detail_templates_dirs, "detail template dirs");

        Ok(Self {
            home_dir,
            cache_dir,
            logs_dir,
            module_dirs,
            extractor_dirs,
            detail_templates_dirs,
            num_backend_threads: value.num_backend_threads,
            num_evaluator_threads: value.num_evaluator_threads,
        })
    }
}

fn wrap_opt(dir: PathBuf, name: &str) -> Option<PathBuf> {
    if create_dir(&dir, name) {
        Some(dir)
    } else {
        None
    }
}

fn create_dirs(mut dirs: Vec<PathBuf>, name: &str) -> Vec<PathBuf> {
    dirs.retain(|dir| create_dir(dir, name));
    dirs
}

fn create_dir(dir: &Path, name: &str) -> bool {
    match std::fs::create_dir_all(dir) {
        Ok(_) => {
            info!("Ensured existence of {name} {}", dir.display());
            true
        }
        Err(err) => {
            error!("Failed to create {name} {}: {err}", dir.display(),);
            false
        }
    }
}
