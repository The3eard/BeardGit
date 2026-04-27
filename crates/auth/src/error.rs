//! Error types for the auth crate.

/// Errors that can occur during authentication or credential operations.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Failed to read machine ID: {0}")]
    MachineId(String),
    #[error("Credential file error: {0}")]
    CredentialFile(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Invalid token: {0}")]
    InvalidToken(String),
}
