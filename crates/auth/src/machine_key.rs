//! Machine-derived encryption key for credential storage.
//!
//! Retrieves a platform-specific machine identifier and derives a 32-byte AES-256 key
//! using HKDF-SHA256. The key is deterministic for a given machine — same machine always
//! produces the same key, different machine produces a different key.
//!
//! ## Platform support
//!
//! | Platform | Source |
//! |----------|--------|
//! | macOS | `IOPlatformUUID` via `ioreg` |
//! | Linux | `/etc/machine-id` |
//! | Windows | `MachineGuid` from registry |

use hkdf::Hkdf;
use sha2::Sha256;

use crate::error::AuthError;

// SALT VERSIONING:
// v1 (2026-03-27): Initial version. HKDF-SHA256 with machine ID as IKM.
//
// To add a new version:
// 1. Add the new salt constant (e.g., SALT_V2)
// 2. Update derive_key() to use the new salt
// 3. Add a migration path in CredentialFile::read() that tries the old
//    key on decryption failure, re-encrypts with the new key if successful
// 4. Document the change here with date and reason
const SALT_V1: &[u8] = b"beardgit-credential-store-v1";
const INFO: &[u8] = b"encryption-key";

/// Derive a 32-byte AES-256 key from raw key material using HKDF-SHA256.
pub fn derive_key(ikm: &[u8]) -> Result<[u8; 32], AuthError> {
    let hk = Hkdf::<Sha256>::new(Some(SALT_V1), ikm);
    let mut key = [0u8; 32];
    hk.expand(INFO, &mut key)
        .map_err(|e| AuthError::Encryption(format!("HKDF expand failed: {e}")))?;
    Ok(key)
}

/// Read the machine's unique identifier. Platform-specific.
pub fn get_machine_id() -> Result<String, AuthError> {
    platform::read_machine_id()
}

/// Derive the encryption key from the current machine's ID.
pub fn derive_machine_key() -> Result<[u8; 32], AuthError> {
    let id = get_machine_id()?;
    derive_key(id.as_bytes())
}

#[cfg(target_os = "macos")]
mod platform {
    use crate::error::AuthError;

    pub fn read_machine_id() -> Result<String, AuthError> {
        let output = std::process::Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .map_err(|e| AuthError::MachineId(format!("Failed to run ioreg: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("IOPlatformUUID")
                && let Some(uuid) = line.split('"').nth(3)
            {
                return Ok(uuid.to_string());
            }
        }
        Err(AuthError::MachineId(
            "IOPlatformUUID not found in ioreg output".to_string(),
        ))
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use crate::error::AuthError;

    pub fn read_machine_id() -> Result<String, AuthError> {
        std::fs::read_to_string("/etc/machine-id")
            .map(|s| s.trim().to_string())
            .map_err(|e| AuthError::MachineId(format!("Failed to read /etc/machine-id: {e}")))
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use crate::error::AuthError;

    pub fn read_machine_id() -> Result<String, AuthError> {
        let output = std::process::Command::new("reg")
            .args([
                "query",
                r"HKLM\SOFTWARE\Microsoft\Cryptography",
                "/v",
                "MachineGuid",
            ])
            .output()
            .map_err(|e| AuthError::MachineId(format!("Failed to query registry: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("MachineGuid") {
                if let Some(guid) = line.split_whitespace().last() {
                    return Ok(guid.to_string());
                }
            }
        }
        Err(AuthError::MachineId(
            "MachineGuid not found in registry output".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_deterministic() {
        let key1 = derive_key(b"test-machine-id").unwrap();
        let key2 = derive_key(b"test-machine-id").unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_inputs_different_keys() {
        let key1 = derive_key(b"machine-a").unwrap();
        let key2 = derive_key(b"machine-b").unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_length() {
        let key = derive_key(b"any-input").unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_get_machine_id_returns_nonempty() {
        let id = get_machine_id().unwrap();
        assert!(!id.is_empty());
    }
}
