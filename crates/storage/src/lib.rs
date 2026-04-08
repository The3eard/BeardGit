//! Storage crate for BeardGit.
//!
//! Provides SQLite-backed commit caching, JSON application configuration,
//! and a unified error type for all storage operations.

pub mod commits_cache;
pub mod config;
pub mod database;
pub mod error;
pub mod theme;

pub use commits_cache::CachedCommit;
pub use config::{AppConfig, GraphColumnConfig};
pub use database::Database;
pub use error::StorageError;
pub use theme::{Theme, ThemeError, ThemeMeta};
