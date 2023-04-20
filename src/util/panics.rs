use std::io::Write;

use cursive::reexports::log::error;

use crate::util::paths::KGV_HOME_DIR;

pub(crate) trait OptionExt<T> {
    fn unwrap_or_log(self) -> T;
}

impl<T> OptionExt<T> for Option<T> {
    #[inline]
    #[track_caller]
    fn unwrap_or_log(self) -> T {
        match self {
            Some(val) => val,
            None => failed_option("Unwrapping Option::None"),
        }
    }
}

pub(crate) trait ResultExt<T, E> {
    fn unwrap_or_log(self) -> T
    where
        E: std::fmt::Debug;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    #[inline]
    #[track_caller]
    fn unwrap_or_log(self) -> T
    where
        E: std::fmt::Debug,
    {
        match self {
            Ok(t) => t,
            Err(e) => failed_result("Unwrapping Err", &e),
        }
    }
}

#[inline(never)]
#[cold]
#[track_caller]
fn failed_result(msg: &str, value: &dyn std::fmt::Debug) -> ! {
    let location = std::panic::Location::caller();
    let message = format!(
        "{}:{}:{} :: {msg}: {value:?}",
        location.file(),
        location.line(),
        location.column()
    );

    let additional_message = match write_to_panics(&message) {
        Ok(_) => "".to_string(),
        Err(err) => {
            error!("Failed writing an error to panic: {}", err);
            format!("; Failed writing an error to panic: {err}")
        }
    };

    error!("{}: {:?}{}", msg, &value, additional_message);
    panic!("{}: {:?}{}", msg, &value, additional_message);
}

#[inline(never)]
#[cold]
#[track_caller]
fn failed_option(msg: &str) -> ! {
    let location = std::panic::Location::caller();
    let message = format!(
        "{}:{}:{} :: {msg}",
        location.file(),
        location.line(),
        location.column()
    );

    let additional_message = match write_to_panics(&message) {
        Ok(_) => "".to_string(),
        Err(err) => {
            error!("Failed writing an error to panic: {}", err);
            format!("; Failed writing an error to panic: {err}")
        }
    };

    error!("{}{}", message, additional_message);
    panic!("{}{}", message, additional_message);
}

fn write_to_panics(message: &str) -> anyhow::Result<()> {
    let panics_file = KGV_HOME_DIR.join("panics.log");

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(panics_file)
        .unwrap();

    writeln!(file, "{message}")?;

    Ok(())
}
