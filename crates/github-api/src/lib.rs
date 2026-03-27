//! GitHub REST API client and [`CiProvider`] implementation for BeardGit.
//!
//! Provides [`GitHubProvider`], which implements the unified [`provider::CiProvider`]
//! trait by delegating to the internal [`GitHubClient`] HTTP client and normalizing
//! GitHub Actions API responses into the shared provider types.

pub mod client;
pub mod provider;
pub mod types;

pub use client::{ApiError, GitHubClient};
pub use provider::GitHubProvider;
