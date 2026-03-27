//! Background task runner for BeardGit.
//!
//! Spawns CLI commands as async tasks with real-time output streaming,
//! lifecycle tracking, and optional cancellation. Decoupled from Tauri
//! via the [`TaskEventSink`] trait.

pub mod manager;
pub mod sink;
pub mod types;

pub use manager::TaskManager;
pub use sink::TaskEventSink;
pub use types::*;
