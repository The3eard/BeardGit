//! SQLite-backed persistence for the Requests panel: global library,
//! execution history, and per-project state (active env, last open
//! request, divider position).
//!
//! This crate owns its own SQLite file (`requests.db`) — it does not
//! share a connection with the main `storage` crate so its migrations
//! and schema evolve independently.

pub mod database;
pub mod error;
pub mod global;
pub mod history;
pub mod project_state;

pub use database::RequestsDatabase;
pub use error::RequestsStoreError;
pub use global::{GlobalCollection, GlobalItem};
pub use history::{HISTORY_CAP_PER_SOURCE, HistoryEntry, HistoryInsert};
pub use project_state::ProjectState;
