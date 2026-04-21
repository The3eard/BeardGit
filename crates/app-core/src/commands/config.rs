//! Git config listing, setting, and user identity commands.

use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// List all config entries at the given scope ("local", "global", or "system").
#[tauri::command]
pub fn list_config(
    scope: git_engine::ConfigScope,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::ConfigEntry>, String> {
    with_active_repo(&state, |repo| {
        repo.list_config(scope).map_err(|e| e.to_string())
    })
}

/// Set a config key to a value at the given scope.
#[tauri::command]
pub fn set_config(
    scope: git_engine::ConfigScope,
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.set_config(scope, &key, &value)
            .map_err(|e| e.to_string())
    })
}

/// Remove a config key at the given scope.
#[tauri::command]
pub fn unset_config(
    scope: git_engine::ConfigScope,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.unset_config(scope, &key).map_err(|e| e.to_string())
    })
}

/// Add a new value for a config key at the given scope (multi-value append).
#[tauri::command]
pub fn add_config(
    scope: git_engine::ConfigScope,
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.add_config(scope, &key, &value)
            .map_err(|e| e.to_string())
    })
}

/// Return the current user's identities (emails and names) for author highlighting.
///
/// Collects `user.email` and `user.name` from git config plus any connected
/// provider user emails, display names, and usernames. Returns a deduplicated,
/// lowercased list of all identity strings.
#[tauri::command]
pub fn get_user_identities(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut identities: Vec<String> = Vec::new();

    // Git config email and name from active repo
    if let Ok(email) = with_active_repo(&state, |repo| {
        let config = repo.inner().config().map_err(|e| e.to_string())?;
        config.get_string("user.email").map_err(|e| e.to_string())
    }) {
        let lower = email.to_lowercase();
        if !lower.is_empty() {
            identities.push(lower);
        }
    }
    if let Ok(name) = with_active_repo(&state, |repo| {
        let config = repo.inner().config().map_err(|e| e.to_string())?;
        config.get_string("user.name").map_err(|e| e.to_string())
    }) {
        let lower = name.to_lowercase();
        if !lower.is_empty() {
            identities.push(lower);
        }
    }

    // Connected provider identities (email, display_name, username)
    if let Ok(providers) = state.providers.lock() {
        for conn in providers.iter() {
            if let Some(ref email) = conn.user.email {
                let lower = email.to_lowercase();
                if !lower.is_empty() {
                    identities.push(lower);
                }
            }
            let display = conn.user.display_name.to_lowercase();
            if !display.is_empty() {
                identities.push(display);
            }
            let username = conn.user.username.to_lowercase();
            if !username.is_empty() {
                identities.push(username);
            }
        }
    }

    identities.sort();
    identities.dedup();
    Ok(identities)
}

#[cfg(test)]
mod tests {
    //! Drive local-scope git config CRUD against a fixture repo. Global /
    //! system scopes would mutate the tester's ~/.gitconfig — we don't
    //! touch those here.

    use git_engine::test_support::create_repo_with_n_commits;
    use git_engine::{ConfigScope, Repository};

    #[test]
    fn set_then_list_config_roundtrips_entry() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.set_config(ConfigScope::Local, "beardgit.test-key", "hello")
            .unwrap();
        let entries = repo.list_config(ConfigScope::Local).unwrap();
        assert!(
            entries
                .iter()
                .any(|e| e.key == "beardgit.test-key" && e.value == "hello"),
            "expected beardgit.test-key=hello, got {entries:?}"
        );
    }

    #[test]
    fn unset_config_removes_existing_entry() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.set_config(ConfigScope::Local, "beardgit.doomed", "v")
            .unwrap();
        repo.unset_config(ConfigScope::Local, "beardgit.doomed")
            .unwrap();
        let entries = repo.list_config(ConfigScope::Local).unwrap();
        assert!(
            !entries.iter().any(|e| e.key == "beardgit.doomed"),
            "unset should have removed the key, got {entries:?}"
        );
    }

    #[test]
    fn add_config_appends_additional_value() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        repo.add_config(ConfigScope::Local, "beardgit.list", "one")
            .unwrap();
        repo.add_config(ConfigScope::Local, "beardgit.list", "two")
            .unwrap();
        let entries = repo.list_config(ConfigScope::Local).unwrap();
        let values: Vec<_> = entries
            .iter()
            .filter(|e| e.key == "beardgit.list")
            .map(|e| e.value.clone())
            .collect();
        assert!(
            values.contains(&"one".to_string()) && values.contains(&"two".to_string()),
            "both appended values should be present, got {values:?}"
        );
    }

    #[test]
    fn unset_config_on_missing_key_errors() {
        let (_tmp, path) = create_repo_with_n_commits(1);
        let repo = Repository::open(&path).unwrap();
        let err = repo
            .unset_config(ConfigScope::Local, "beardgit.never-set")
            .err();
        assert!(
            err.is_some(),
            "unsetting a missing key should error (mirrors `git config`)"
        );
    }
}
