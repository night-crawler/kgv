use crate::util::panics::ResultExt;

pub trait KanalSenderExt<T> {
    fn send_unwrap(&self, msg: T);
}

impl<T> KanalSenderExt<T> for kanal::Sender<T> {
    #[inline]
    #[track_caller]
    fn send_unwrap(&self, msg: T) {
        // let location = std::panic::Location::caller();
        self.send(msg).unwrap_or_log();
    }
}
