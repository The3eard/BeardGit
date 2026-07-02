//! Core application logic for BeardGit.
//!
//! This crate exposes Tauri command handlers and the shared application state
//! used by the `src-tauri` shell. It wires together `git-engine`,
//! `graph-builder`, `gitlab-api`, `auth`, and `storage` into the IPC surface
//! consumed by the Svelte frontend.

pub mod ai_background;
pub mod ai_commands;
pub mod commands;
pub mod event_sink;
pub mod ipc_error;
pub mod state;
pub mod task_commands;
pub mod task_events;
pub mod terminal_commands;
pub mod terminal_sink;
