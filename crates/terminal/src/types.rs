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
    ///
    /// Historically this field was also abused to smuggle a full command
    /// line (`"/abs/path/to/bin --flag value"`) because no `args` field
    /// existed. `portable-pty` treats the shell value as one filename, so
    /// that usage failed with `ENOENT` on any non-trivial invocation —
    /// see `args` below for the right way to pass arguments.
    pub shell: Option<String>,
    /// Arguments to pass to the shell/command. Each element is a single
    /// argv entry; no tokenisation happens on spaces.
    pub args: Vec<String>,
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

/// Payload for the `terminal-cwd-changed` Tauri event.
#[derive(Clone, Debug, Serialize)]
pub struct TerminalCwdChangedEvent {
    pub session_id: SessionId,
    /// The new working directory path.
    pub cwd: String,
}

/// Payload for the `terminal-process-changed` Tauri event.
#[derive(Clone, Debug, Serialize)]
pub struct TerminalProcessChangedEvent {
    pub session_id: SessionId,
    /// Name of the foreground process, or `None` if detection failed or the
    /// shell itself is in the foreground.
    pub process_name: Option<String>,
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
    /// Failed to resize an already-running PTY session.
    #[error("failed to resize terminal: {0}")]
    ResizeFailed(String),
    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to detect a shell on this system.
    #[error("no shell detected")]
    NoShellDetected,
}
