use k8s_openapi::serde_json;
use rhai::EvalAltResult;

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

    #[error("Error: {0}")]
    AnyhowError(#[from] anyhow::Error),

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
}

impl From<&'static str> for KgvError {
    fn from(value: &'static str) -> Self {
        Self::StrError(value)
    }
}
