use std::collections::VecDeque;
use std::path::PathBuf;

use cursive::reexports::log::error;

pub fn scan_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut queue = VecDeque::from_iter(roots.iter().cloned());
    let mut files = vec![];
    while let Some(path) = queue.pop_front() {
        if path.is_dir() {
            match std::fs::read_dir(&path) {
                Ok(dir) => {
                    for dir_entry in dir {
                        match dir_entry {
                            Ok(dir_entry) => {
                                let path = dir_entry.path();
                                if path.is_file() {
                                    files.push(path);
                                } else {
                                    queue.push_back(path);
                                }
                            }
                            Err(err) => {
                                error!("Failed to get dir entry: {err}");
                            }
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to read dir {}: {}", path.display(), err);
                }
            }
        } else {
            files.push(path.clone());
        }
    }

    files
}
