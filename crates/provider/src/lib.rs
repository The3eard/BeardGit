//! Unified provider abstraction for git hosting services.
//!
//! This crate defines the [`CiProvider`] trait and all shared types used by
//! both the GitLab and GitHub provider implementations. It contains no HTTP
//! logic — only the contract and data structures.
//!
//! ## Architecture
//!
//! - [`CiProvider`] — async trait that GitLab and GitHub providers implement
//! - [`CiRun`], [`CiJob`], [`CiStage`] — normalized CI/CD types
//! - [`CiStatus`] — unified status enum across both providers
//! - [`ProviderUser`], [`Project`] — common identity and project types
//! - [`ProviderError`] — provider-agnostic error type
//! - [`parse_remote_url`] — detect provider from git remote URL
//!
//! ## Trait-crate purity
//!
//! This crate must stay free of runtime dependencies (no `reqwest`, `tokio`
//! runtimes, `tauri`, `hyper`, or similar). It is the public contract shared
//! between the frontend, implementation crates, and the app core. CI contains
//! a grep-based guard that fails the build if forbidden deps leak in.

pub mod error;
pub mod http_helpers;
pub mod kind;
pub mod log_preprocessor;
pub mod traits;
pub mod types;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

pub use error::ProviderError;
pub use kind::{ProviderKind, parse_remote_url};
pub use traits::CiProvider;
pub use types::{
    CiFilters, CiJob, CiJobStep, CiRun, CiRunDetail, CiStage, CiStatus, ConnectedProvider, Project,
    ProviderStatusResponse, ProviderUser, TriggerResult, TriggerWorkflowInput, Workflow,
    WorkflowState,
};

#[cfg(any(test, feature = "mock"))]
pub use mock::MockCiProvider;
