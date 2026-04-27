//! Git config read/write operations.
//!
//! Extends [`Repository`] with methods to list, set, unset, and add git
//! configuration entries at local, global, and system scope. Uses the git CLI
//! for all operations to ensure consistent behaviour with the user's git setup.

use serde::{Deserialize, Serialize};

use crate::error::GitError;
use crate::repository::Repository;

/// The scope of a git configuration entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigScope {
    /// Repository-level config (`.git/config`).
    Local,
    /// User-level config (`~/.gitconfig`).
    Global,
    /// System-wide config (`/etc/gitconfig`).
    System,
}

impl ConfigScope {
    /// Return the `--<scope>` flag used by `git config`.
    fn flag(self) -> &'static str {
        match self {
            ConfigScope::Local => "--local",
            ConfigScope::Global => "--global",
            ConfigScope::System => "--system",
        }
    }
}

/// A single git configuration entry with its key, value, and scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    /// Fully qualified key (e.g. `"core.autocrlf"`).
    pub key: String,
    /// The value as a string.
    pub value: String,
    /// Which config file this entry comes from.
    pub scope: ConfigScope,
}

impl Repository {
    /// List all config entries at the given scope.
    ///
    /// Uses `git config --list --null` for reliable parsing of multi-line values.
    /// The `--null` flag separates entries with NUL bytes, making parsing unambiguous.
    pub fn list_config(&self, scope: ConfigScope) -> Result<Vec<ConfigEntry>, GitError> {
        let result = self.git_cmd(&["config", scope.flag(), "--list", "--null"])?;

        // git config exits non-zero if the scope file doesn't exist (e.g. no system config)
        if !result.success {
            return Ok(Vec::new());
        }

        Ok(parse_config_null(&result.stdout, scope))
    }

    /// Set a configuration key to a value at the given scope.
    ///
    /// If the key already exists, it is overwritten. If it doesn't exist, it is created.
    pub fn set_config(&self, scope: ConfigScope, key: &str, value: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["config", scope.flag(), key, value])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Remove a configuration key at the given scope.
    ///
    /// If the key has multiple values, all values are removed (`--unset-all`).
    pub fn unset_config(&self, scope: ConfigScope, key: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["config", scope.flag(), "--unset-all", key])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }

    /// Add a new value for a configuration key at the given scope.
    ///
    /// Unlike [`set_config`], this does not replace existing values — it appends
    /// a new entry. Useful for multi-valued keys like `remote.origin.fetch`.
    pub fn add_config(&self, scope: ConfigScope, key: &str, value: &str) -> Result<(), GitError> {
        let result = self.git_cmd(&["config", scope.flag(), "--add", key, value])?;
        if result.success {
            Ok(())
        } else {
            Err(GitError::CliError(result.stderr))
        }
    }
}

/// Parse the output of `git config --list --null` into [`ConfigEntry`] structs.
///
/// The `--null` output format uses NUL (`\0`) to separate entries, and each
/// entry is formatted as `key\nvalue` (newline between key and value).
fn parse_config_null(output: &str, scope: ConfigScope) -> Vec<ConfigEntry> {
    output
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            // Split on first newline: key\nvalue (value may contain newlines)
            let (key, value) = match entry.find('\n') {
                Some(pos) => (&entry[..pos], &entry[pos + 1..]),
                None => (entry, ""),
            };
            ConfigEntry {
                key: key.to_string(),
                value: value.to_string(),
                scope,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Parser unit tests ──────────────────────────────────────────────────

    #[test]
    fn test_parse_config_null_basic() {
        let output = "user.name\nJohn Doe\0user.email\njohn@example.com\0";
        let entries = parse_config_null(output, ConfigScope::Global);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "user.name");
        assert_eq!(entries[0].value, "John Doe");
        assert_eq!(entries[0].scope, ConfigScope::Global);
        assert_eq!(entries[1].key, "user.email");
        assert_eq!(entries[1].value, "john@example.com");
    }

    #[test]
    fn test_parse_config_null_multiline_value() {
        let output = "alias.lg\nlog --oneline\n--graph\0";
        let entries = parse_config_null(output, ConfigScope::Local);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "alias.lg");
        assert_eq!(entries[0].value, "log --oneline\n--graph");
    }

    #[test]
    fn test_parse_config_null_empty() {
        let entries = parse_config_null("", ConfigScope::System);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_config_null_key_only() {
        // Some boolean keys have no value
        let output = "core.bare\0";
        let entries = parse_config_null(output, ConfigScope::Local);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, "core.bare");
        assert_eq!(entries[0].value, "");
    }

    #[test]
    fn test_config_scope_flags() {
        assert_eq!(ConfigScope::Local.flag(), "--local");
        assert_eq!(ConfigScope::Global.flag(), "--global");
        assert_eq!(ConfigScope::System.flag(), "--system");
    }

    // ── Integration tests ──────────────────────────────────────────────────

    fn create_test_repo() -> (tempfile::TempDir, crate::repository::Repository) {
        let tmp = tempfile::tempdir().unwrap();
        let git_repo = git2::Repository::init(tmp.path()).unwrap();
        {
            let mut cfg = git_repo.config().unwrap();
            cfg.set_str("user.name", "Test").unwrap();
            cfg.set_str("user.email", "test@test.com").unwrap();
        }
        std::fs::write(tmp.path().join("file.txt"), "hello").unwrap();
        {
            let mut index = git_repo.index().unwrap();
            index.add_path(std::path::Path::new("file.txt")).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = git_repo.find_tree(tree_id).unwrap();
            let sig = git_repo.signature().unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        drop(git_repo);
        let repo = crate::repository::Repository::open(tmp.path()).unwrap();
        (tmp, repo)
    }

    #[test]
    fn test_list_config_local() {
        let (_tmp, repo) = create_test_repo();
        let entries = repo.list_config(ConfigScope::Local).unwrap();
        assert!(!entries.is_empty());
        // Should contain user.name we set in init
        assert!(
            entries
                .iter()
                .any(|e| e.key == "user.name" && e.value == "Test")
        );
    }

    #[test]
    fn test_set_and_list_config() {
        let (_tmp, repo) = create_test_repo();
        repo.set_config(ConfigScope::Local, "test.key", "test-value")
            .unwrap();
        let entries = repo.list_config(ConfigScope::Local).unwrap();
        assert!(
            entries
                .iter()
                .any(|e| e.key == "test.key" && e.value == "test-value")
        );
    }

    #[test]
    fn test_unset_config() {
        let (_tmp, repo) = create_test_repo();
        repo.set_config(ConfigScope::Local, "test.removeme", "value")
            .unwrap();
        repo.unset_config(ConfigScope::Local, "test.removeme")
            .unwrap();
        let entries = repo.list_config(ConfigScope::Local).unwrap();
        assert!(!entries.iter().any(|e| e.key == "test.removeme"));
    }

    #[test]
    fn test_set_config_overwrites() {
        let (_tmp, repo) = create_test_repo();
        repo.set_config(ConfigScope::Local, "test.key", "old")
            .unwrap();
        repo.set_config(ConfigScope::Local, "test.key", "new")
            .unwrap();
        let entries = repo.list_config(ConfigScope::Local).unwrap();
        let vals: Vec<_> = entries
            .iter()
            .filter(|e| e.key == "test.key")
            .map(|e| e.value.as_str())
            .collect();
        assert_eq!(vals, vec!["new"]);
    }
}
