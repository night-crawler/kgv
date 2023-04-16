use std::fmt::Display;

use cursive::reexports::log;
use cursive::reexports::log::{logger, Level, error};
use handlebars::TemplateError;
use k8s_openapi::serde_json;
use rhai::EvalAltResult;
use strum_macros::Display;

#[derive(Clone, Debug, thiserror::Error)]
pub enum EvalError {
    #[error("DateTime parse error: {0}")]
    IoError(#[from] chrono::ParseError),
}

#[derive(Debug, thiserror::Error)]
pub enum KgvError {
    #[error("Compile Error for {0} {1}: {2} {3}")]
    ContentCompileError(String, String, String, anyhow::Error),

    #[error("Engine JSON parse error {0}: {1}")]
    EngineJsonParseError(String, EvalAltResult),

    #[error("Duplicate GVK: {0}")]
    DuplicateGvkError(String),

    #[error("Type conversion error: {0}")]
    TypeConversionError(String),

    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML Serialization error: {0}")]
    SerdeYamlError(#[from] serde_yaml::Error),

    #[error("YAML Serialization error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("YAML Serialization error: {0}")]
    EngineError(#[from] Box<EvalAltResult>),

    #[error("Static string error: {0}")]
    StrError(&'static str),

    #[error("Mutex is poisoned: {0}")]
    MutexPoisoned(String),

    #[error("Template render error : {0}")]
    TemplateRenderError(#[from] TemplateError),
}

impl From<&'static str> for KgvError {
    fn from(value: &'static str) -> Self {
        Self::StrError(value)
    }
}

#[derive(Display, Debug, thiserror::Error)]
pub enum LogError {
    Info(
        String,
        Option<anyhow::Error>,
        &'static std::panic::Location<'static>,
    ),
    Warn(
        String,
        Option<anyhow::Error>,
        &'static std::panic::Location<'static>,
    ),
    Error(
        String,
        Option<anyhow::Error>,
        &'static std::panic::Location<'static>,
    ),
    Debug(
        String,
        Option<anyhow::Error>,
        &'static std::panic::Location<'static>,
    ),
}

impl LogError {
    #[inline]
    #[track_caller]
    pub fn log_error<S>(str: S) -> anyhow::Result<()>
    where
        S: Into<String>,
    {
        let location = std::panic::Location::caller();
        Err(Self::Error(str.into(), None, location).into())
    }

    #[inline]
    #[track_caller]
    pub fn log_warn<R, S>(str: S) -> anyhow::Result<R>
    where
        S: Into<String>,
    {
        let location = std::panic::Location::caller();
        Err(Self::Warn(str.into(), None, location).into())
    }

    #[inline]
    #[track_caller]
    pub fn log_info<R, S>(str: S) -> anyhow::Result<R>
    where
        S: Into<String>,
    {
        let location = std::panic::Location::caller();
        Err(Self::Info(str.into(), None, location).into())
    }

    pub fn log(&self) {
        let mut builder = log::Record::builder();
        let location = self.get_location();
        let level = self.get_level();

        builder
            .line(location.line().into())
            .file(location.file().into())
            .level(level);

        let (text, err) = match self {
            LogError::Info(text, err, _)
            | LogError::Warn(text, err, _)
            | LogError::Error(text, err, _)
            | LogError::Debug(text, err, _) => (text, err),
        };

        let logger = logger();
        if let Some(err) = err {
            logger.log(
                &builder
                    .args(format_args!("{}; Error: {}", text, err))
                    .build(),
            );
        } else {
            logger.log(&builder.args(format_args!("{}", text)).build());
        }
    }

    fn get_location(&self) -> &&'static std::panic::Location<'static> {
        match self {
            LogError::Info(_, _, location)
            | LogError::Warn(_, _, location)
            | LogError::Error(_, _, location)
            | LogError::Debug(_, _, location) => location,
        }
    }

    fn get_level(&self) -> Level {
        match self {
            LogError::Info(_, _, _) => Level::Info,
            LogError::Warn(_, _, _) => Level::Warn,
            LogError::Error(_, _, _) => Level::Error,
            LogError::Debug(_, _, _) => Level::Debug,
        }
    }
}

pub trait LogErrorResultExt<T, E> {
    fn to_log_info<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
    fn to_log_warn<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
    fn to_log_error<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
    fn to_log_debug<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
}

impl<T, E> LogErrorResultExt<T, E> for Result<T, E>
where
    E: Into<anyhow::Error> + Send + Sync + 'static,
{
    #[inline]
    #[track_caller]
    fn to_log_info<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();
        self.map_err(|err| LogError::Info(cb(&err).to_string(), Some(err.into()), location))
    }

    #[inline]
    #[track_caller]
    fn to_log_warn<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();

        self.map_err(|err| LogError::Warn(cb(&err).to_string(), Some(err.into()), location))
    }

    #[inline]
    #[track_caller]
    fn to_log_error<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();

        self.map_err(|err| LogError::Error(cb(&err).to_string(), Some(err.into()), location))
    }

    #[inline]
    #[track_caller]
    fn to_log_debug<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();
        self.map_err(|err| LogError::Debug(cb(&err).to_string(), Some(err.into()), location))
    }
}

pub trait LogErrorOptionExt<T> {
    fn to_log_info<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
    fn to_log_warn<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
    fn to_log_error<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
    fn to_log_debug<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static;
}

impl<T> LogErrorOptionExt<T> for Option<T> {
    #[inline]
    #[track_caller]
    fn to_log_info<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();

        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Info(cb().to_string(), None, location)),
        }
    }

    #[inline]
    #[track_caller]
    fn to_log_warn<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();

        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Warn(cb().to_string(), None, location)),
        }
    }

    #[inline]
    #[track_caller]
    fn to_log_error<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();

        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Error(cb().to_string(), None, location)),
        }
    }

    #[inline]
    #[track_caller]
    fn to_log_debug<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        let location = std::panic::Location::caller();

        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Debug(cb().to_string(), None, location)),
        }
    }
}
