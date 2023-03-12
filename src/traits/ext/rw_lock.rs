use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use util::panics::ResultExt;

use crate::util;

pub trait RwLockExt<T> {
    fn read_unwrap(&self) -> RwLockReadGuard<'_, T>;
    fn write_unwrap(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T> RwLockExt<T> for RwLock<T> {
    fn read_unwrap(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_log()
    }

    fn write_unwrap(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_log()
    }
}
