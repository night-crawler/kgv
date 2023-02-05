use util::panics::ResultExt;

use crate::reexports::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use crate::util;
use crate::util::error::KgvError;

pub trait RwLockExt<T> {
    fn read_unwrap(&self) -> RwLockReadGuard<'_, T>;
    fn write_unwrap(&self) -> RwLockWriteGuard<'_, T>;

    fn read_sync(&self) -> Result<RwLockReadGuard<'_, T>, KgvError>;
    fn write_sync(&self) -> Result<RwLockWriteGuard<'_, T>, KgvError>;
}

impl<T> RwLockExt<T> for RwLock<T> {
    fn read_unwrap(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_log()
    }

    fn write_unwrap(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_log()
    }

    fn read_sync(&self) -> Result<RwLockReadGuard<'_, T>, KgvError> {
        self.read()
            .map_err(|err| KgvError::MutexPoisoned(err.to_string()))
    }

    fn write_sync(&self) -> Result<RwLockWriteGuard<'_, T>, KgvError> {
        self.write()
            .map_err(|err| KgvError::MutexPoisoned(err.to_string()))
    }
}
