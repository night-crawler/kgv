use std::path::PathBuf;

use lazy_static::lazy_static;

pub const SELF_NAME: &str = ".kgv";

lazy_static! {
    pub static ref HOME_DIR: PathBuf = home::home_dir().unwrap();
    pub static ref KGV_HOME_DIR: PathBuf = HOME_DIR.join(SELF_NAME);
}
