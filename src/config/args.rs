use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = r###"kgv
"###
)]
pub struct Args {
    /// A directory where kgv configs are located. If home directory can't be accessed or does not
    /// exist, default column layout will be used.
    #[arg(long, default_value_os_t = get_home_dir())]
    home_dir: PathBuf,

    /// A directory where kgv cache is located
    #[arg(long, default_value_os_t = get_cache_dir())]
    cache_dir: PathBuf,

    /// A directory where kgv rhai modules are located
    #[arg(long, default_values_os_t = get_module_dirs())]
    module_dirs: Vec<PathBuf>,

    /// A directory where kgv list view column definitions are described
    #[arg(long, default_value_os_t = get_col_defs_dir())]
    col_defs_dir: PathBuf,

    /// A directory where kgv detail view columns are described
    #[arg(long, default_value_os_t = detail_templates_dir())]
    detail_templates_dir: PathBuf,

    /// Number of tokio worker threads used to communicate with k8s cluster.
    #[arg(long, default_value_t = 2)]
    num_backend_threads: usize,

    /// Number of rhai engine evaluator threads. Each engine instance created per thread.
    #[arg(long, default_value_t = 2)]
    num_evaluator_threads: usize,

    #[command(subcommand)]
    action: Action,
}


#[derive(clap::Subcommand, Debug)]
enum Action {
    /// Generate a configuration for a Group Version Kind with defaults
    Generate {

    },
    /// Run extraction on a given entity using current configuration
    Extract {
        /// A path to a fixture containing the resource content in YAML foormat
        #[arg(long)]
        fixture: PathBuf,
    },
}

fn get_home_dir() -> PathBuf {
    home::home_dir().unwrap().join(".kgv")
}

fn get_cache_dir() -> PathBuf {
    get_home_dir().join("cache")
}

fn get_module_dirs() -> Vec<PathBuf> {
    vec![get_home_dir().join("modules")]
}

fn get_col_defs_dir() -> PathBuf {
    get_home_dir().join("views").join("list")
}

fn detail_templates_dir() -> PathBuf {
    get_home_dir().join("views").join("detail")
}