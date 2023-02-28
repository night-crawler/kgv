use std::path::PathBuf;

use lazy_static::lazy_static;

const SELF_NAME: &str = ".kgv";

lazy_static! {
    pub static ref HOME_DIR: PathBuf = home::home_dir().unwrap();
    pub static ref KGV_HOME_DIR: PathBuf = HOME_DIR.join(SELF_NAME);
    pub static ref LOGS_DIR: PathBuf = KGV_HOME_DIR.join("logs");
    pub static ref CACHE_DIR: PathBuf = KGV_HOME_DIR.join("cache");
    pub static ref COLUMNS_FILE: PathBuf = KGV_HOME_DIR.join("columns.yaml");
}

pub fn create_all_paths() -> anyhow::Result<()> {
    std::fs::create_dir_all(KGV_HOME_DIR.clone())?;
    std::fs::create_dir_all(LOGS_DIR.clone())?;
    std::fs::create_dir_all(CACHE_DIR.clone())?;

    Ok(())
}
