//! Tauri command handlers exposed to the Svelte frontend via IPC.
//!
//! Commands are organized into feature-based modules. The `helpers` module
//! contains shared types and utility functions used across all command modules.
//! Everything is re-exported here so callers (e.g. `src-tauri/src/lib.rs` and
//! `ai_commands.rs`) can reference `commands::function_name` unchanged.

mod helpers;

mod advanced;
mod bisect;
mod branch;
mod ci;
mod clean;
mod cli_auth;
mod commit;
mod config;
mod conflict;
mod diff;
mod gitignore;
mod graph;
mod issues;
mod logging;
mod mr_pr;
mod patch;
mod project;
mod provider_auth;
mod reflog;
mod releases;
mod remote;
mod repository;
mod settings;
mod staging;
mod stash;
mod submodule;
mod tag;
mod theme;
mod worktree;

// Re-export all public items so the rest of the crate sees a flat namespace.
pub use advanced::*;
pub use bisect::*;
pub use branch::*;
pub use ci::*;
pub use clean::*;
pub use cli_auth::*;
pub use commit::*;
pub use config::*;
pub use conflict::*;
pub use diff::*;
pub use gitignore::*;
pub use graph::*;
pub(crate) use helpers::get_active_project_path;
pub use helpers::{GraphViewport, ProjectInfo, RecentRepo, RemoteInfo, RepoInfo};
pub use issues::*;
pub use logging::*;
pub use mr_pr::*;
pub use patch::*;
pub use project::*;
pub use provider_auth::*;
pub use reflog::*;
pub use releases::*;
pub use remote::*;
pub use repository::*;
pub use settings::*;
pub use staging::*;
pub use stash::*;
pub use submodule::*;
pub use tag::*;
pub use theme::*;
pub use worktree::*;
