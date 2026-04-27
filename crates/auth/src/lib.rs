//! Authentication and credential management for BeardGit.
//!
//! Provides encrypted credential storage using machine-derived AES-256-GCM encryption,
//! and PAT (Personal Access Token) validation against provider APIs.
//!
//! ## Architecture
//!
//! - [`CredentialStore`] — public API for storing/retrieving credentials
//! - [`Credential`] — token + provider pair (uses [`provider::ProviderKind`] for the provider field)
//! - [`validate_gitlab_pat`] — validates a PAT against a GitLab instance
//! - [`validate_github_pat`] — validates a PAT against a GitHub instance
//!
//! ## Storage
//!
//! Credentials are encrypted with AES-256-GCM and stored in `~/.config/beardgit/credentials.enc`.
//! The encryption key is derived from the machine's unique ID via HKDF-SHA256, so the file
//! is useless if copied to another machine.

pub(crate) mod credential_file;
pub mod credential_store;
pub mod error;
pub mod machine_key;
pub mod pat;

pub use credential_store::{Credential, CredentialStore};
pub use error::AuthError;
pub use pat::{validate_github_pat, validate_gitlab_pat};
