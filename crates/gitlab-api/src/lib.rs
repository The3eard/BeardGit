//! GitLab REST API v4 client and [`CiProvider`] implementation for BeardGit.
//!
//! Provides [`GitLabProvider`], which implements the unified [`provider::CiProvider`]
//! trait by delegating to the internal [`GitLabClient`] HTTP client and normalizing
//! GitLab-specific types into the shared provider types.

pub mod client;
pub mod jobs;
pub mod pipelines;
pub mod provider;
pub mod types;

pub use client::{ApiError, GitLabClient};
pub use provider::GitLabProvider;
pub use types::*;
