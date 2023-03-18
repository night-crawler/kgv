use std::fmt::Display;

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

    #[error("Mutex is poisoned : {0}")]
    MutexPoisoned(String),
}

impl From<&'static str> for KgvError {
    fn from(value: &'static str) -> Self {
        Self::StrError(value)
    }
}

#[derive(Display, Debug, thiserror::Error)]
pub enum LogError {
    Info(String, Option<anyhow::Error>),
    Warn(String, Option<anyhow::Error>),
    Error(String, Option<anyhow::Error>),
    Debug(String, Option<anyhow::Error>),
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
    fn to_log_info<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|err| LogError::Info(cb(&err).to_string(), Some(err.into())))
    }

    fn to_log_warn<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|err| LogError::Warn(cb(&err).to_string(), Some(err.into())))
    }

    fn to_log_error<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|err| LogError::Error(cb(&err).to_string(), Some(err.into())))
    }

    fn to_log_debug<C>(self, cb: impl FnOnce(&E) -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|err| LogError::Debug(cb(&err).to_string(), Some(err.into())))
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
    fn to_log_info<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Info(cb().to_string(), None)),
        }
    }

    fn to_log_warn<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Warn(cb().to_string(), None)),
        }
    }

    fn to_log_error<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Error(cb().to_string(), None)),
        }
    }

    fn to_log_debug<C>(self, cb: impl FnOnce() -> C) -> Result<T, LogError>
    where
        C: Display + Send + Sync + 'static,
    {
        match self {
            Some(value) => Ok(value),
            None => Err(LogError::Debug(cb().to_string(), None)),
        }
    }
}

