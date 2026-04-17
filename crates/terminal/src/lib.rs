//! Terminal session manager — spawns, manages, and communicates with PTY sessions.
//!
//! Uses `portable-pty` for cross-platform PTY support. Decoupled from Tauri
//! via the [`TerminalEventSink`] trait (same pattern as `task-runner`).

pub mod manager;
pub mod osc;
pub mod process;
pub mod shell;
pub mod sink;
pub mod types;

pub use manager::TerminalManager;
pub use sink::TerminalEventSink;
pub use types::{
    SessionId, TerminalConfig, TerminalCwdChangedEvent, TerminalError, TerminalExitEvent,
    TerminalOutputEvent, TerminalProcessChangedEvent,
};
