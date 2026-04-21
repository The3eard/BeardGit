//! Disk-backed persistence for computed graph layouts.
//!
//! One JSON file per repository, keyed by a hash of the repo's absolute
//! path, storing the last-computed [`GraphLayout`] alongside the refs state
//! that produced it. Loaders compare a freshly-computed `cache_key` against
//! the stored one; mismatch (or any IO / parse / version error) is treated
//! as a silent miss so the caller can fall back to a live walk + compute.

use std::path::{Path, PathBuf};

use graph_builder::GraphLayout;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current on-disk schema revision. A mismatch on load is treated as a miss.
pub const SCHEMA_VERSION: u32 = 1;

/// A single repo's cached graph layout along with the state that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutCacheEntry {
    /// Revision of the on-disk format; see [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// SHA-256 hex of `(repo_path, head_oid, sorted refs)` — the identity of
    /// the repo state that produced the embedded [`GraphLayout`].
    pub cache_key: String,
    /// Absolute path to the repo whose layout is cached.
    pub repo_path: String,
    /// HEAD OID at the time the layout was computed.
    pub head_oid: String,
    /// ISO-8601 timestamp of when the entry was written (informational only).
    pub generated_at: String,
    /// The computed layout that callers will reuse on a cache hit.
    pub layout: GraphLayout,
}

/// Compute the identity key for a repository's current state.
///
/// Combines the repo path, HEAD OID, and the sorted list of `(ref_name, oid)`
/// pairs into a single SHA-256 hex string. Any change to HEAD, any ref
/// create/delete, or any ref movement produces a different key.
pub fn compute_cache_key(repo_path: &str, head_oid: &str, refs: &[(String, String)]) -> String {
    let mut sorted: Vec<&(String, String)> = refs.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let joined: String = sorted
        .iter()
        .map(|(n, o)| format!("{n}={o}"))
        .collect::<Vec<_>>()
        .join(",");
    let material = format!("{repo_path}:{head_oid}:{joined}");
    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Resolve the on-disk cache file path for a given repo under a config dir.
///
/// Mirrors the convention used by [`crate::project_cache`]: the caller passes
/// the BeardGit-specific config directory (usually `<os_config_dir>/beardgit`)
/// and this module appends its own `layouts/` subdirectory. Files live at
/// `<config_dir>/layouts/<sha256(repo_path)>.json`.
pub fn layout_cache_path(config_dir: &Path, repo_path: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(repo_path.as_bytes());
    let name = format!("{:x}.json", hasher.finalize());
    config_dir.join("layouts").join(name)
}

/// Persist a [`LayoutCacheEntry`] to disk, creating parent directories as needed.
///
/// Overwrites any existing file for the same `repo_path`. Returns IO errors
/// verbatim; callers typically log at `warn!` and discard on failure.
pub fn save_layout_cache(config_dir: &Path, entry: &LayoutCacheEntry) -> std::io::Result<()> {
    let path = layout_cache_path(config_dir, &entry.repo_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(entry).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

/// Load a cached [`LayoutCacheEntry`] for the given repo, if one exists.
///
/// Returns `Ok(None)` on any of: missing file, JSON parse failure, or
/// schema-version mismatch. Only unexpected IO errors (e.g. permission
/// denied reading an existing file) bubble up as `Err`.
pub fn load_layout_cache(
    config_dir: &Path,
    repo_path: &str,
) -> std::io::Result<Option<LayoutCacheEntry>> {
    let path = layout_cache_path(config_dir, repo_path);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    let entry: LayoutCacheEntry = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Ok(None), // corrupt → silent miss
    };
    if entry.schema_version != SCHEMA_VERSION {
        return Ok(None);
    }
    Ok(Some(entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_layout() -> GraphLayout {
        GraphLayout {
            nodes: Vec::new(),
            lane_count: 0,
            lane_segments: Vec::new(),
            merge_curves: Vec::new(),
            head_lane: None,
        }
    }

    fn sample_entry(repo_path: &str, cache_key: &str) -> LayoutCacheEntry {
        LayoutCacheEntry {
            schema_version: SCHEMA_VERSION,
            cache_key: cache_key.to_string(),
            repo_path: repo_path.to_string(),
            head_oid: "aaaa".to_string(),
            generated_at: "2026-04-20T00:00:00Z".to_string(),
            layout: sample_layout(),
        }
    }

    #[test]
    fn compute_cache_key_is_stable() {
        let refs = vec![
            ("refs/heads/main".to_string(), "aaaa".to_string()),
            ("refs/heads/dev".to_string(), "bbbb".to_string()),
        ];
        let k1 = compute_cache_key("/repo", "aaaa", &refs);
        let k2 = compute_cache_key("/repo", "aaaa", &refs);
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 64, "sha256 hex is 64 chars");
    }

    #[test]
    fn compute_cache_key_changes_with_head() {
        let refs = vec![("refs/heads/main".to_string(), "aaaa".to_string())];
        assert_ne!(
            compute_cache_key("/repo", "aaaa", &refs),
            compute_cache_key("/repo", "bbbb", &refs),
        );
    }

    #[test]
    fn compute_cache_key_changes_with_refs() {
        let r1 = vec![("refs/heads/main".to_string(), "aaaa".to_string())];
        let r2 = vec![
            ("refs/heads/main".to_string(), "aaaa".to_string()),
            ("refs/heads/dev".to_string(), "bbbb".to_string()),
        ];
        assert_ne!(
            compute_cache_key("/repo", "aaaa", &r1),
            compute_cache_key("/repo", "aaaa", &r2),
        );
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let entry = sample_entry("/some/repo", "key-1");
        save_layout_cache(tmp.path(), &entry).unwrap();
        let loaded = load_layout_cache(tmp.path(), "/some/repo")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.cache_key, entry.cache_key);
        assert_eq!(loaded.repo_path, entry.repo_path);
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn load_returns_none_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let got = load_layout_cache(tmp.path(), "/none").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn load_returns_none_when_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = "/repo";
        let path = layout_cache_path(tmp.path(), repo_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{not json").unwrap();
        let got = load_layout_cache(tmp.path(), repo_path).unwrap();
        assert!(
            got.is_none(),
            "corrupt file should be treated as miss, not panic/error"
        );
    }

    #[test]
    fn load_returns_none_when_schema_version_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = "/repo";
        let path = layout_cache_path(tmp.path(), repo_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let bad = serde_json::json!({
            "schema_version": 99,
            "cache_key": "x",
            "repo_path": repo_path,
            "head_oid": "x",
            "generated_at": "2026-04-20T00:00:00Z",
            "layout": {
                "nodes": [],
                "lane_count": 0,
                "lane_segments": [],
                "merge_curves": [],
                "head_lane": null,
            },
        });
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        let got = load_layout_cache(tmp.path(), repo_path).unwrap();
        assert!(got.is_none());
    }
}
