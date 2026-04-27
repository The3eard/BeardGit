//! Per-project snapshot cache for instant UI on project switch.
//!
//! Each project gets a small JSON file in `<config_dir>/project-cache/`
//! keyed by a hash of the project path. The snapshot stores the last-known
//! git state so the frontend can display badges, titlebar, and tooltip data
//! instantly without waiting for a full status computation.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::StorageError;

/// Per-project cached git state for instant UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    /// Absolute path to the project's working directory.
    pub path: String,
    /// Current branch name (None if detached HEAD).
    pub head_branch: Option<String>,
    /// Commits ahead of upstream.
    pub ahead: usize,
    /// Commits behind upstream.
    pub behind: usize,
    /// Staged file count.
    pub staged: usize,
    /// Modified (unstaged) file count.
    pub unstaged: usize,
    /// Untracked file count.
    pub untracked: usize,
    /// Conflicted file count.
    pub conflicted: usize,
    /// Stash entry count.
    pub stash_count: usize,
    /// Total change count (staged + unstaged + untracked).
    pub change_count: usize,
    /// Persisted commit-graph viewport slice — populated on save so a
    /// cold start can paint the canvas synchronously. `#[serde(default)]`
    /// keeps older on-disk snapshots (pre-Phase-8) deserialising cleanly.
    #[serde(default)]
    pub graph_viewport_cache: Option<GraphViewportCache>,
}

/// Persisted commit-graph viewport slice (see spec's cache shape).
///
/// Stored opaquely as `serde_json::Value` for `nodes` so the Rust side
/// doesn't need to mirror the `LayoutNode` struct one-to-one; the TS
/// layer owns the exact shape and drives validation. Size ≈ 60 KB.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphViewportCache {
    /// Opaque `LayoutNode[]` JSON — Rust stores and forwards only.
    pub nodes: serde_json::Value,
    /// Total commit count at cache time.
    pub total_count: usize,
    /// HEAD OID at cache time; coarse staleness check.
    pub head_oid: String,
    /// First visible commit OID — primary scroll anchor for reconciliation.
    pub top_oid: String,
    /// Scroll offset captured at cache time.
    pub offset: usize,
    /// Epoch milliseconds when the cache was written; compared against
    /// the frontend's 7-day TTL on load.
    pub cached_at: i64,
}

/// Compute the cache filename for a project path using std DefaultHasher.
fn cache_filename(project_path: &str) -> String {
    let mut hasher = DefaultHasher::new();
    project_path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Return the cache directory path, creating it if needed.
fn cache_dir(config_dir: &Path) -> Result<PathBuf, StorageError> {
    let dir = config_dir.join("project-cache");
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Load a project snapshot from the cache. Returns `None` if not found.
pub fn load_snapshot(
    config_dir: &Path,
    project_path: &str,
) -> Result<Option<ProjectSnapshot>, StorageError> {
    let dir = cache_dir(config_dir)?;
    let file = dir.join(format!("{}.json", cache_filename(project_path)));
    if !file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&file)?;
    let snapshot: ProjectSnapshot = serde_json::from_str(&content)?;
    Ok(Some(snapshot))
}

/// Save a project snapshot to the cache.
pub fn save_snapshot(config_dir: &Path, snapshot: &ProjectSnapshot) -> Result<(), StorageError> {
    let dir = cache_dir(config_dir)?;
    let file = dir.join(format!("{}.json", cache_filename(&snapshot.path)));
    let content = serde_json::to_string_pretty(snapshot)?;
    fs::write(&file, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_filename_deterministic() {
        let a = cache_filename("/Users/test/project");
        let b = cache_filename("/Users/test/project");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn test_cache_filename_different_paths() {
        let a = cache_filename("/Users/test/project-a");
        let b = cache_filename("/Users/test/project-b");
        assert_ne!(a, b);
    }

    #[test]
    fn test_save_and_load_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot = ProjectSnapshot {
            path: "/Users/test/project".to_string(),
            head_branch: Some("main".to_string()),
            ahead: 2,
            behind: 0,
            staged: 1,
            unstaged: 3,
            untracked: 5,
            conflicted: 0,
            stash_count: 1,
            change_count: 9,
            graph_viewport_cache: None,
        };
        save_snapshot(tmp.path(), &snapshot).unwrap();
        let loaded = load_snapshot(tmp.path(), "/Users/test/project").unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.path, "/Users/test/project");
        assert_eq!(loaded.ahead, 2);
        assert_eq!(loaded.change_count, 9);
        assert_eq!(loaded.head_branch, Some("main".to_string()));
        assert!(loaded.graph_viewport_cache.is_none());
    }

    #[test]
    fn test_load_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let loaded = load_snapshot(tmp.path(), "/nonexistent").unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_overwrite_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let snapshot1 = ProjectSnapshot {
            path: "/Users/test/project".to_string(),
            head_branch: Some("main".to_string()),
            ahead: 1,
            behind: 0,
            staged: 0,
            unstaged: 0,
            untracked: 0,
            conflicted: 0,
            stash_count: 0,
            change_count: 0,
            graph_viewport_cache: None,
        };
        save_snapshot(tmp.path(), &snapshot1).unwrap();

        let snapshot2 = ProjectSnapshot {
            path: "/Users/test/project".to_string(),
            head_branch: Some("feature".to_string()),
            ahead: 5,
            behind: 2,
            staged: 3,
            unstaged: 1,
            untracked: 0,
            conflicted: 0,
            stash_count: 0,
            change_count: 4,
            graph_viewport_cache: None,
        };
        save_snapshot(tmp.path(), &snapshot2).unwrap();

        let loaded = load_snapshot(tmp.path(), "/Users/test/project")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.head_branch, Some("feature".to_string()));
        assert_eq!(loaded.ahead, 5);
    }

    /// Pre-Phase-8 snapshots on disk lack the `graph_viewport_cache`
    /// field. The `#[serde(default)]` attribute must let them
    /// deserialise without error so existing users don't lose their
    /// cached state on upgrade.
    #[test]
    fn test_legacy_snapshot_without_viewport_field_deserialises() {
        let legacy_json = r#"{
            "path": "/Users/test/legacy",
            "head_branch": "main",
            "ahead": 0,
            "behind": 0,
            "staged": 0,
            "unstaged": 0,
            "untracked": 0,
            "conflicted": 0,
            "stash_count": 0,
            "change_count": 0
        }"#;
        let snap: ProjectSnapshot = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(snap.path, "/Users/test/legacy");
        assert!(snap.graph_viewport_cache.is_none());
    }

    /// Round-trip a snapshot with a populated viewport slice to verify
    /// the new field persists through save → load.
    #[test]
    fn test_snapshot_with_viewport_cache_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        let snap = ProjectSnapshot {
            path: "/Users/test/vp".to_string(),
            head_branch: Some("main".to_string()),
            ahead: 0,
            behind: 0,
            staged: 0,
            unstaged: 0,
            untracked: 0,
            conflicted: 0,
            stash_count: 0,
            change_count: 0,
            graph_viewport_cache: Some(GraphViewportCache {
                nodes: serde_json::json!([{ "oid": "abc123" }]),
                total_count: 42,
                head_oid: "abc123".to_string(),
                top_oid: "abc123".to_string(),
                offset: 7,
                cached_at: 1_700_000_000_000,
            }),
        };
        save_snapshot(tmp.path(), &snap).unwrap();
        let loaded = load_snapshot(tmp.path(), "/Users/test/vp")
            .unwrap()
            .unwrap();
        let cache = loaded.graph_viewport_cache.expect("cache should persist");
        assert_eq!(cache.total_count, 42);
        assert_eq!(cache.top_oid, "abc123");
        assert_eq!(cache.offset, 7);
        assert_eq!(cache.cached_at, 1_700_000_000_000);
    }
}
