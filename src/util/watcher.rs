use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use cursive::reexports::log::error;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

type Builder<T> = dyn FnMut(&[PathBuf]) -> T + Send + Sync;

pub struct LazyWatcher<T> {
    create: Box<Builder<T>>,
    data: T,
    flag: Arc<AtomicBool>,
    watch_paths: Vec<PathBuf>,
    _watchers: Vec<RecommendedWatcher>,
}

impl<T: Send + Sync> LazyWatcher<T> {
    pub fn new(
        watch_paths: Vec<PathBuf>,
        mut create: impl FnMut(&[PathBuf]) -> T + Send + Sync + 'static,
    ) -> Result<Self, notify::Error> {
        let mut watchers = vec![];
        let flag = Arc::new(AtomicBool::new(false));

        for path in watch_paths.iter() {
            let flag = Arc::clone(&flag);
            let mut watcher = notify::recommended_watcher(move |res| match res {
                Ok(_event) => flag.store(true, std::sync::atomic::Ordering::Release),
                Err(e) => {
                    error!("Watcher error: {:?}", e)
                }
            })?;

            watcher.watch(path, RecursiveMode::Recursive)?;
            watchers.push(watcher);
        }

        let data = create(&watch_paths);

        Ok(Self {
            data,
            _watchers: watchers,
            create: Box::new(create),
            flag,
            watch_paths,
        })
    }

    pub fn get(&mut self) -> &T {
        if self.flag.swap(false, std::sync::atomic::Ordering::AcqRel) {
            self.data = (self.create)(&self.watch_paths);
        }

        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch() {
        let temp_dir = tempfile::tempdir().unwrap().into_path();
        let mut counter = 0;
        let mut watcher = LazyWatcher::new(vec![temp_dir.clone()], move |_| {
            counter += 1;
            counter
        })
        .unwrap();
        assert_eq!(watcher.get(), &1);

        while watcher.get() == &1 {
            tempfile::tempfile_in(temp_dir.clone()).unwrap();
        }
        assert_eq!(watcher.get(), &2);
    }
}
