//! Error types for the requests-store crate.

use thiserror::Error;

/// All errors that can occur during requests-store operations.
#[derive(Debug, Error)]
pub enum RequestsStoreError {
    /// A rusqlite / SQLite error.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// A filesystem I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON (de)serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
