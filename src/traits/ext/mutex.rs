use std::sync::{Mutex, MutexGuard};

use crate::util::panics::ResultExt;

pub trait MutexExt<T> {
    fn lock_unwrap(&self) -> MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for Mutex<T> {
    fn lock_unwrap(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_log()
    }
}
