use std::path::{Path, PathBuf};

use cursive::reexports::log::warn;
use lazy_static::lazy_static;

pub(crate) const SELF_NAME: &str = ".kgv";

lazy_static! {
    pub(crate) static ref HOME_DIR: PathBuf = home::home_dir().unwrap();
    pub(crate) static ref KGV_HOME_DIR: PathBuf = HOME_DIR.join(SELF_NAME);
}

fn resolve_path_internal(origin: &Path, rel_path: &Path) -> PathBuf {
    if rel_path.is_absolute() {
        return rel_path.into();
    }

    if let Some(parent) = origin.parent() {
        parent.join(rel_path)
    } else {
        rel_path.into()
    }
}

pub(crate) fn resolve_path(origin: &Path, rel_path: &Path) -> PathBuf {
    let path = resolve_path_internal(origin, rel_path);
    match path.canonicalize() {
        Ok(path) => path,
        Err(err) => {
            warn!("Could not canonicalize path {}: {}", path.display(), err);
            path
        }
    }
}
