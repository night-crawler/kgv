use crate::util::error::LogError;
use cursive::reexports::log::error;

pub mod backend_dispatch_ext;
pub mod backend_dispatcher;
pub mod send_helper_ext;
pub mod ui_dispatch_ext;
pub mod ui_dispatcher;

pub fn log_signal_result(result: anyhow::Result<()>, signal_name: &str) {
    if let Err(err) = result {
        if let Some(err) = err.downcast_ref::<LogError>() {
            err.log();
        } else {
            error!("Failed to dispatch signal {signal_name}: {err}");
        }
    }
}
