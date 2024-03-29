use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use cursive::reexports::log::error;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::reexports::sync::{Mutex, RwLock};
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::rw_lock::RwLockExt;

type Builder<T> = dyn FnMut(&[PathBuf]) -> T + Send + Sync;

pub(crate) struct LazyWatcher<T> {
    builder: Mutex<Box<Builder<T>>>,
    data: RwLock<Arc<T>>,
    flag: Arc<AtomicBool>,
    version: AtomicUsize,
    watch_paths: Vec<PathBuf>,
    _watchers: Vec<RecommendedWatcher>,
}

impl<T: Send + Sync> LazyWatcher<T> {
    pub(crate) fn new(
        watch_paths: Vec<PathBuf>,
        mut builder: impl FnMut(&[PathBuf]) -> T + Send + Sync + 'static,
    ) -> Result<Self, notify::Error> {
        let mut watchers = vec![];
        let flag = Arc::new(AtomicBool::new(false));

        for path in watch_paths.iter() {
            let flag = Arc::clone(&flag);
            let mut watcher = notify::recommended_watcher(move |res| match res {
                Ok(_event) => flag.store(true, Ordering::Release),
                Err(e) => {
                    error!("Watcher error: {:?}", e)
                }
            })?;

            watcher.watch(path, RecursiveMode::Recursive)?;
            watchers.push(watcher);
        }

        let data = builder(&watch_paths);

        Ok(Self {
            version: AtomicUsize::new(0),
            data: RwLock::new(Arc::new(data)),
            _watchers: watchers,
            builder: Mutex::new(Box::new(builder)),
            flag,
            watch_paths,
        })
    }

    pub(crate) fn value(&self) -> Arc<T> {
        if self.flag.swap(false, Ordering::AcqRel) {
            self.version.fetch_add(1, Ordering::AcqRel);
            *self.data.write_unwrap() = Arc::new(self.build());
        }

        self.data.read_unwrap().clone()
    }

    pub(crate) fn build(&self) -> T {
        (self.builder.lock_unwrap())(&self.watch_paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch() {
        let temp_dir = tempfile::tempdir().unwrap().into_path();
        let mut counter = 0;
        let watcher = LazyWatcher::new(vec![temp_dir.clone()], move |_| {
            counter += 1;
            counter
        })
        .unwrap();
        assert_eq!(*watcher.value(), 1);

        while *watcher.value() == 1 {
            tempfile::tempfile_in(temp_dir.clone()).unwrap();
        }
        assert!(*watcher.value() >= 2);
    }
}
