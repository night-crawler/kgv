pub(crate) trait CloningCallbackExt {
    fn cloning<R>(&self, cb: impl Fn(Self) -> R) -> R
    where
        Self: Sized + Clone;
}

impl<T> CloningCallbackExt for T {
    fn cloning<R>(&self, cb: impl Fn(Self) -> R) -> R
    where
        Self: Sized + Clone,
    {
        cb(self.clone())
    }
}
