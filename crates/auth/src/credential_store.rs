//! Provider-agnostic credential store backed by an encrypted file.
//!
//! # Example (production)
//!
//! ```no_run
//! use auth::{CredentialStore, Credential};
//! use provider::ProviderKind;
//! use std::path::Path;
//!
//! let store = CredentialStore::new(Path::new("/home/user/.config/beardgit")).unwrap();
//!
//! // Store a GitLab token
//! store.store_credential("https://gitlab.com", &Credential {
//!     token: "glpat-xxxx".to_string(),
//!     provider: ProviderKind::GitLab,
//! }).unwrap();
//!
//! // Retrieve it later
//! if let Some(cred) = store.get_credential("https://gitlab.com").unwrap() {
//!     println!("Token for {}: {}", cred.provider == ProviderKind::GitLab, cred.token);
//! }
//! ```

use provider::ProviderKind;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::credential_file::CredentialFile;
use crate::error::AuthError;
use crate::machine_key;

/// A stored credential: an authentication token paired with its provider type.
///
/// `Debug` is implemented manually to redact the token — without that a stray
/// `tracing::debug!(?cred)` or `dbg!(cred)` somewhere downstream would write
/// the raw PAT into the rotating log file.
#[derive(Clone, Serialize, Deserialize)]
pub struct Credential {
    pub token: String,
    pub provider: ProviderKind,
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credential")
            .field("token", &"[REDACTED]")
            .field("provider", &self.provider)
            .finish()
    }
}

/// Encrypted credential store backed by `~/.config/beardgit/credentials.enc`.
///
/// Uses AES-256-GCM encryption with a machine-derived key. The store is
/// provider-agnostic — it can hold credentials for any number of instances
/// across different providers (GitLab, GitHub, etc.).
///
/// Note: The read-modify-write cycle in `store_credential` and `delete_credential`
/// is not atomic. For the MVP single-credential use case this is safe, but if
/// concurrent writes become possible, wrap this in a `Mutex`.
pub struct CredentialStore {
    file: CredentialFile,
    key: [u8; 32],
}

impl CredentialStore {
    /// Production constructor — derives key from the real machine ID.
    pub fn new(config_dir: &Path) -> Result<Self, AuthError> {
        let key = machine_key::derive_machine_key()?;
        Ok(Self {
            file: CredentialFile::new(config_dir),
            key,
        })
    }

    /// Test constructor — accepts an explicit key to avoid platform machine ID calls.
    #[cfg(test)]
    fn with_key(config_dir: &Path, key: [u8; 32]) -> Self {
        Self {
            file: CredentialFile::new(config_dir),
            key,
        }
    }

    /// Store a credential for a given instance URL.
    pub fn store_credential(
        &self,
        instance_url: &str,
        credential: &Credential,
    ) -> Result<(), AuthError> {
        let mut map = self.file.read(&self.key)?;
        let value = serde_json::to_value(credential).map_err(|e| {
            AuthError::CredentialFile(format!("Failed to serialize credential: {e}"))
        })?;
        map.insert(instance_url.to_string(), value);
        self.file.write(&self.key, &map)
    }

    /// Retrieve a credential for a given instance URL. Returns None if not found.
    pub fn get_credential(&self, instance_url: &str) -> Result<Option<Credential>, AuthError> {
        let map = self.file.read(&self.key)?;
        match map.get(instance_url) {
            Some(value) => {
                let credential: Credential =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        AuthError::CredentialFile(format!("Failed to deserialize credential: {e}"))
                    })?;
                Ok(Some(credential))
            }
            None => Ok(None),
        }
    }

    /// Delete a credential for a given instance URL. No-op if not found.
    pub fn delete_credential(&self, instance_url: &str) -> Result<(), AuthError> {
        let mut map = self.file.read(&self.key)?;
        if map.remove(instance_url).is_some() {
            self.file.write(&self.key, &map)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0xAA; 32]
    }

    fn gitlab_credential() -> Credential {
        Credential {
            token: "glpat-abc123".to_string(),
            provider: ProviderKind::GitLab,
        }
    }

    fn github_credential() -> Credential {
        Credential {
            token: "ghp_xyz789".to_string(),
            provider: ProviderKind::GitHub,
        }
    }

    #[test]
    fn test_store_and_retrieve_credential() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::with_key(dir.path(), test_key());
        let cred = gitlab_credential();

        store.store_credential("https://gitlab.com", &cred).unwrap();
        let result = store.get_credential("https://gitlab.com").unwrap();

        assert!(result.is_some());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.token, "glpat-abc123");
        assert_eq!(retrieved.provider, ProviderKind::GitLab);
    }

    #[test]
    fn test_delete_credential() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::with_key(dir.path(), test_key());

        store
            .store_credential("https://gitlab.com", &gitlab_credential())
            .unwrap();
        store.delete_credential("https://gitlab.com").unwrap();

        let result = store.get_credential("https://gitlab.com").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::with_key(dir.path(), test_key());

        store.delete_credential("https://nonexistent.com").unwrap();
    }

    #[test]
    fn test_multiple_instances() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::with_key(dir.path(), test_key());

        store
            .store_credential("https://gitlab.com", &gitlab_credential())
            .unwrap();
        store
            .store_credential("https://github.com", &github_credential())
            .unwrap();

        let gl = store.get_credential("https://gitlab.com").unwrap().unwrap();
        assert_eq!(gl.token, "glpat-abc123");
        assert_eq!(gl.provider, ProviderKind::GitLab);

        let gh = store.get_credential("https://github.com").unwrap().unwrap();
        assert_eq!(gh.token, "ghp_xyz789");
        assert_eq!(gh.provider, ProviderKind::GitHub);
    }

    #[test]
    fn test_missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::with_key(dir.path(), test_key());

        let result = store.get_credential("https://gitlab.com").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_corrupted_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::with_key(dir.path(), test_key());

        std::fs::write(
            dir.path().join("credentials.enc"),
            b"this is garbage data that is definitely long enough to pass nonce check",
        )
        .unwrap();

        let result = store.get_credential("https://gitlab.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_overwrite_existing_credential() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::with_key(dir.path(), test_key());

        store
            .store_credential("https://gitlab.com", &gitlab_credential())
            .unwrap();

        let updated = Credential {
            token: "glpat-updated999".to_string(),
            provider: ProviderKind::GitLab,
        };
        store
            .store_credential("https://gitlab.com", &updated)
            .unwrap();

        let result = store.get_credential("https://gitlab.com").unwrap().unwrap();
        assert_eq!(result.token, "glpat-updated999");
    }
}
