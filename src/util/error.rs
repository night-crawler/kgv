#[derive(Clone, Debug, thiserror::Error)]
pub enum EvalError {
    #[error("DateTime parse error: {0}")]
    IoError(#[from] chrono::ParseError),
}

#[derive(Debug, thiserror::Error)]
pub enum KgvError {
    #[error("Compile Error for {0} {1}: {2} {3}")]
    ContentCompileError(String, String, String, anyhow::Error),

    #[error("Duplicate GVK: {0}")]
    DuplicateGvkError(String),

    #[error("Error: {0}")]
    AnyhowError(#[from] anyhow::Error),

    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("YAML Serialization error: {0}")]
    SerdeYamlError(#[from] serde_yaml::Error),
}
