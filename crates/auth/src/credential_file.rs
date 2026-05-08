//! Low-level encrypted file I/O for the credential store.
//!
//! This module is `pub(crate)` — external consumers should use [`crate::credential_store::CredentialStore`].
//!
//! ## File format
//!
//! ```text
//! [12-byte nonce][AES-256-GCM ciphertext + 16-byte auth tag]
//! ```
//!
//! The plaintext is a JSON object mapping instance URLs to credential data.
//! A fresh random nonce is generated on every write.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::AuthError;

const NONCE_LEN: usize = 12;
const CREDENTIALS_FILENAME: &str = "credentials.enc";

/// Handles encryption, decryption, and file I/O for the credentials file.
pub struct CredentialFile {
    path: PathBuf,
}

impl CredentialFile {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            path: config_dir.join(CREDENTIALS_FILENAME),
        }
    }

    /// Read and decrypt the credentials file. Returns empty map if file doesn't exist.
    pub fn read(&self, key: &[u8; 32]) -> Result<HashMap<String, serde_json::Value>, AuthError> {
        let data = match std::fs::read(&self.path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(HashMap::new());
            }
            Err(e) => {
                return Err(AuthError::CredentialFile(format!(
                    "Failed to read {}: {e}",
                    self.path.display()
                )));
            }
        };

        if data.len() < NONCE_LEN {
            return Err(AuthError::CredentialFile(
                "Credential file too short to contain a nonce".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| AuthError::Encryption(format!("Failed to create cipher: {e}")))?;

        let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
            AuthError::CredentialFile(
                "Failed to decrypt credentials file — wrong key or corrupted file".to_string(),
            )
        })?;

        let map: HashMap<String, serde_json::Value> =
            serde_json::from_slice(&plaintext).map_err(|e| {
                AuthError::CredentialFile(format!("Failed to parse decrypted JSON: {e}"))
            })?;

        Ok(map)
    }

    /// Encrypt and write the credentials map to disk. Creates parent dirs if needed.
    pub fn write(
        &self,
        key: &[u8; 32],
        data: &HashMap<String, serde_json::Value>,
    ) -> Result<(), AuthError> {
        let json = serde_json::to_vec(data)
            .map_err(|e| AuthError::CredentialFile(format!("Failed to serialize JSON: {e}")))?;

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| AuthError::Encryption(format!("Failed to create cipher: {e}")))?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, json.as_ref())
            .map_err(|e| AuthError::Encryption(format!("Encryption failed: {e}")))?;

        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AuthError::CredentialFile(format!(
                    "Failed to create directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

        // Write nonce || ciphertext
        let mut output = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);

        // Atomic write: stage to a sibling temp file, fsync, then rename.
        // A crash mid-write would otherwise truncate `credentials.enc` and
        // wipe every stored PAT — `std::fs::write` is not atomic.
        let tmp_path = self.path.with_extension("enc.tmp");
        std::fs::write(&tmp_path, &output).map_err(|e| {
            AuthError::CredentialFile(format!("Failed to write {}: {e}", tmp_path.display()))
        })?;

        // Tighten the file mode on Unix so other local users cannot read the
        // ciphertext. The encryption is sound on its own, but the machine ID
        // used to derive the key is also readable by every local user, so a
        // 0644 file is effectively a same-user binding rather than a same-
        // machine binding. 0600 closes that gap.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600));
        }

        std::fs::rename(&tmp_path, &self.path).map_err(|e| {
            // Best-effort cleanup if rename fails — the temp file is
            // useless on its own and would leak otherwise.
            let _ = std::fs::remove_file(&tmp_path);
            AuthError::CredentialFile(format!(
                "Failed to rename {} -> {}: {e}",
                tmp_path.display(),
                self.path.display()
            ))
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0xAA; 32]
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cf = CredentialFile::new(dir.path());
        let key = test_key();

        let mut data = HashMap::new();
        data.insert(
            "https://gitlab.com".to_string(),
            serde_json::json!({"token": "glpat-abc123", "provider": "gitlab"}),
        );

        cf.write(&key, &data).unwrap();
        let read_back = cf.read(&key).unwrap();

        assert_eq!(data, read_back);
    }

    #[test]
    fn test_missing_file_returns_empty_map() {
        let dir = tempfile::tempdir().unwrap();
        let cf = CredentialFile::new(dir.path());
        let key = test_key();

        let result = cf.read(&key).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_corrupted_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let cf = CredentialFile::new(dir.path());
        let key = test_key();

        std::fs::write(
            dir.path().join("credentials.enc"),
            b"this is garbage data that is long enough",
        )
        .unwrap();

        let result = cf.read(&key);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("decrypt") || err.contains("corrupted"),
            "Expected decryption error, got: {err}"
        );
    }

    #[test]
    fn test_wrong_key_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let cf = CredentialFile::new(dir.path());

        let key_a = [0xAA; 32];
        let key_b = [0xBB; 32];

        let mut data = HashMap::new();
        data.insert("url".to_string(), serde_json::json!("token"));

        cf.write(&key_a, &data).unwrap();
        let result = cf.read(&key_b);
        assert!(result.is_err());
    }

    #[test]
    fn test_atomic_write_leaves_no_temp_file() {
        // After a successful write, only `credentials.enc` should exist —
        // not the `credentials.enc.tmp` staging file.
        let dir = tempfile::tempdir().unwrap();
        let cf = CredentialFile::new(dir.path());
        let key = test_key();

        let mut data = HashMap::new();
        data.insert("k".to_string(), serde_json::json!("v"));
        cf.write(&key, &data).unwrap();

        assert!(dir.path().join(CREDENTIALS_FILENAME).exists());
        assert!(!dir.path().join("credentials.enc.tmp").exists());
    }

    #[test]
    fn test_overwrite_preserves_data_on_success() {
        // Two sequential writes must not corrupt the second's contents.
        let dir = tempfile::tempdir().unwrap();
        let cf = CredentialFile::new(dir.path());
        let key = test_key();

        let mut first = HashMap::new();
        first.insert("a".to_string(), serde_json::json!("1"));
        cf.write(&key, &first).unwrap();

        let mut second = HashMap::new();
        second.insert("b".to_string(), serde_json::json!("2"));
        cf.write(&key, &second).unwrap();

        let read = cf.read(&key).unwrap();
        assert_eq!(read, second);
    }

    #[test]
    fn test_file_too_short() {
        let dir = tempfile::tempdir().unwrap();
        let cf = CredentialFile::new(dir.path());
        let key = test_key();

        std::fs::write(dir.path().join("credentials.enc"), b"short").unwrap();

        let result = cf.read(&key);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }
}
