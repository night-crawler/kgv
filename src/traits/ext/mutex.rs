use cursive::reexports::log::{debug, error};

use crate::reexports::{Mutex, MutexGuard};
use crate::util::error::KgvError;
use crate::util::panics::ResultExt;

pub trait MutexExt<T> {
    fn lock_unwrap(&self) -> MutexGuard<'_, T>;
    fn lock_sync(&self) -> Result<MutexGuard<'_, T>, KgvError>;
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

    #[inline(never)]
    #[track_caller]
    fn lock_sync(&self) -> Result<MutexGuard<'_, T>, KgvError> {
        let location = std::panic::Location::caller();
        match self.lock() {
            Ok(guard) => {
                debug!(
                    "Acquired lock. File: {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                );
                Ok(guard)
            }
            Err(err) => {
                let location = std::panic::Location::caller();
                error!(
                    "Failed to lock mutex: {}; file: {}:{}:{}",
                    err,
                    location.file(),
                    location.line(),
                    location.column()
                );
                Err(KgvError::MutexPoisoned(err.to_string()))
            }
        }
    }
}
