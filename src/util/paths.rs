use std::path::PathBuf;

use lazy_static::lazy_static;

const SELF_NAME: &str = ".kgv";

lazy_static! {
    pub static ref HOME_DIR: PathBuf = home::home_dir().unwrap();
    pub static ref KGV_HOME_DIR: PathBuf = HOME_DIR.join(SELF_NAME);
    pub static ref COLUMNS_FILE: PathBuf = KGV_HOME_DIR.join("columns.yaml");
}
