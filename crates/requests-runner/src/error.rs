//! Error types for the requests-runner crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RequestsError {
    #[error("parse error at line {line}, col {col}: {reason}")]
    Parse {
        line: usize,
        col: usize,
        reason: String,
    },

    #[error("missing secret `{name}` in env `{env}`")]
    MissingSecret { env: String, name: String },

    #[error("unresolved variable `{name}`")]
    UnresolvedVar { name: String },

    #[error("variable cycle detected: {vars:?}")]
    CycleDetected { vars: Vec<String> },

    #[error("network: {0}")]
    Network(String),

    #[error("canceled")]
    Canceled,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("storage error: {0}")]
    Store(#[from] requests_store::RequestsStoreError),
}
