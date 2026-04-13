//! Core types for the terminal session system.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

/// Unique identifier for a terminal session.
pub type SessionId = u64;

/// Configuration for spawning a new terminal session.
#[derive(Debug, Clone)]
pub struct TerminalConfig {
    /// Working directory for the shell.
    pub cwd: PathBuf,
    /// Override the default shell. If `None`, auto-detect from system.
    pub shell: Option<String>,
    /// Additional environment variables to set.
    pub env: HashMap<String, String>,
    /// Initial terminal width in columns.
    pub cols: u16,
    /// Initial terminal height in rows.
    pub rows: u16,
}

/// Payload for the `terminal-output` Tauri event.
#[derive(Clone, Debug, Serialize)]
pub struct TerminalOutputEvent {
    pub session_id: SessionId,
    /// Base64-encoded bytes (raw PTY output).
    pub data: String,
}

/// Payload for the `terminal-exit` Tauri event.
#[derive(Clone, Debug, Serialize)]
pub struct TerminalExitEvent {
    pub session_id: SessionId,
    pub exit_code: Option<u32>,
}

/// Errors that can occur when interacting with the terminal manager.
#[derive(thiserror::Error, Debug)]
pub enum TerminalError {
    /// The requested session ID does not exist.
    #[error("terminal session {0} not found")]
    NotFound(SessionId),
    /// Failed to spawn the shell process.
    #[error("failed to spawn shell: {0}")]
    SpawnFailed(String),
    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to detect a shell on this system.
    #[error("no shell detected")]
    NoShellDetected,
}
