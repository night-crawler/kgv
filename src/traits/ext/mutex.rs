use cursive::reexports::log::{debug, error};

use crate::reexports::sync::{Mutex, MutexGuard};
use crate::util::error::KgvError;
use crate::util::panics::ResultExt;

pub trait MutexExt<T> {
    fn lock_unwrap(&self) -> MutexGuard<'_, T>;
    fn lock_sync(&self) -> Result<MutexGuard<'_, T>, KgvError>;

    fn locking<R>(
        &self,
        f: impl FnOnce(MutexGuard<'_, T>) -> Result<R, anyhow::Error>,
    ) -> Result<R, anyhow::Error> {
        let guard = self.lock_sync()?;
        f(guard)
    }

    fn get_locking<R>(
        &self,
        f: impl FnOnce(MutexGuard<'_, T>) -> R,
    ) -> R {
        let guard = self.lock_unwrap();
        f(guard)
    }
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
