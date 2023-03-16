use std::sync::{Mutex, MutexGuard};

use cursive::reexports::log::debug;

use crate::util::panics::ResultExt;

pub trait MutexExt<T> {
    fn lock_unwrap(&self) -> MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for Mutex<T> {
    #[inline(never)]
    #[track_caller]
    fn lock_unwrap(&self) -> MutexGuard<'_, T> {
        let location = std::panic::Location::caller();
        match self.try_lock() {
            Ok(guard) => {
                debug!(
                    "Acquired lock. File: {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                );
                guard
            }
            Err(err) => {
                let location = std::panic::Location::caller();
                debug!(
                    "Failed to lock mutex: {}; file: {}:{}:{}",
                    err,
                    location.file(),
                    location.line(),
                    location.column()
                );
                self.lock().unwrap_or_log()
            }
        }
    }
}
