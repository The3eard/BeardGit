//! Error types for the storage crate.

use thiserror::Error;

/// All errors that can occur during storage operations.
#[derive(Error, Debug)]
pub enum StorageError {
    /// A rusqlite / SQLite error.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A filesystem I/O error (e.g. reading or writing config files).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// A JSON (de)serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
