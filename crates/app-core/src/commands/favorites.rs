//! Favorite (starred) branches of the active repository.
//!
//! Favorites are a per-repository preference, so they live with the
//! repository — `<repo>/.beardgit/favorites.json`, alongside the requests
//! collections and AI reports — rather than in the app's own config dir. A
//! project that doesn't `.gitignore` `.beardgit/` can therefore commit the
//! file and share the stars with the team; that's the project's call.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::instrument;

use super::helpers::*;
use crate::ipc_error::IpcError;
use crate::state::AppState;

/// On-disk shape of `<repo>/.beardgit/favorites.json`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Favorites {
    /// Starred branch names, local (`beta`) or remote (`origin/main`).
    #[serde(default)]
    branches: Vec<String>,
}

fn favorites_path(project: &Path) -> PathBuf {
    project.join(".beardgit").join("favorites.json")
}

/// Read the starred branches of the repository at `project`.
///
/// A missing file is the normal state of a repo nobody has starred anything
/// in, and reads as an empty list. Unparseable content is an error rather
/// than an empty list: silently treating it as "no favorites" would make the
/// next write erase whatever the file held.
fn read_favorites(project: &Path) -> Result<Vec<String>, IpcError> {
    let path = favorites_path(project);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(IpcError::new(
                "io_error",
                format!("{}: {e}", path.display()),
            ));
        }
    };
    let favorites: Favorites = serde_json::from_str(&raw)
        .map_err(|e| IpcError::new("io_error", format!("{}: {e}", path.display())))?;
    Ok(favorites.branches)
}

/// Replace the starred branches of the repository at `project`, creating
/// `.beardgit/` if this is the first thing to land in it.
fn write_favorites(project: &Path, branches: Vec<String>) -> Result<(), IpcError> {
    let path = favorites_path(project);
    let dir = path
        .parent()
        .ok_or_else(|| IpcError::new("internal", "favorites path has no parent"))?;
    std::fs::create_dir_all(dir)
        .map_err(|e| IpcError::new("io_error", format!("{}: {e}", dir.display())))?;
    let json = serde_json::to_string_pretty(&Favorites { branches })
        .map_err(|e| IpcError::new("internal", e.to_string()))?;
    std::fs::write(&path, json)
        .map_err(|e| IpcError::new("io_error", format!("{}: {e}", path.display())))
}

/// Branch names the user has starred in the active repository.
#[tauri::command]
#[instrument(skip_all, name = "cmd::favorites::get")]
pub fn get_favorite_branches(state: State<'_, AppState>) -> Result<Vec<String>, IpcError> {
    read_favorites(&get_active_project_path(&state)?)
}

/// Replace the starred-branch list of the active repository.
///
/// Takes the whole list rather than one toggle: the file is a handful of
/// short strings, so last-write-wins on it is simpler than reconciling adds
/// and removes, and the frontend already holds the full set.
///
/// No mutation guard — nothing in the git state changed, so a
/// `project-mutated` event would fan out a pointless refresh of every store.
#[tauri::command]
#[instrument(skip_all, fields(count = branches.len()), name = "cmd::favorites::set")]
pub fn set_favorite_branches(
    branches: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), IpcError> {
    write_favorites(&get_active_project_path(&state)?, branches)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn read_favorites_on_a_repo_with_no_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_favorites(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn write_then_read_favorites_roundtrips_and_preserves_order() {
        let dir = tempfile::tempdir().unwrap();
        let stars = names(&["beta", "origin/main", "feat/thing"]);

        write_favorites(dir.path(), stars.clone()).unwrap();

        assert_eq!(read_favorites(dir.path()).unwrap(), stars);
        assert!(dir.path().join(".beardgit/favorites.json").is_file());
    }

    #[test]
    fn write_favorites_creates_the_beardgit_directory() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!dir.path().join(".beardgit").exists());

        write_favorites(dir.path(), names(&["beta"])).unwrap();

        assert!(dir.path().join(".beardgit").is_dir());
    }

    #[test]
    fn write_favorites_replaces_the_previous_list() {
        let dir = tempfile::tempdir().unwrap();
        write_favorites(dir.path(), names(&["beta", "main"])).unwrap();

        write_favorites(dir.path(), names(&["main"])).unwrap();

        assert_eq!(read_favorites(dir.path()).unwrap(), names(&["main"]));
    }

    #[test]
    fn read_favorites_errors_on_unparseable_content() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".beardgit")).unwrap();
        std::fs::write(dir.path().join(".beardgit/favorites.json"), "{not json").unwrap();

        let err = read_favorites(dir.path()).expect_err("must not read as empty");

        assert_eq!(err.code, "io_error");
    }
}
