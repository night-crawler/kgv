use std::path::{Path, PathBuf};

use lazy_static::lazy_static;

pub const SELF_NAME: &str = ".kgv";

lazy_static! {
    pub static ref HOME_DIR: PathBuf = home::home_dir().unwrap();
    pub static ref KGV_HOME_DIR: PathBuf = HOME_DIR.join(SELF_NAME);
}

pub fn resolve_path(origin: &Path, rel_path: PathBuf) -> PathBuf {
    // TODO: canonicalize
    if rel_path.is_absolute() {
        return rel_path;
    }

    if let Some(parent) = origin.parent() {
        parent.join(rel_path)
    } else {
        rel_path
    }
}
