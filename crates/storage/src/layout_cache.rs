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
///
/// Bumped 1 → 2 when the app-core state fingerprint stopped counting reachable
/// commits and switched to O(refs) material (ref hash + HEAD symbolic target +
/// HEAD tree + `.git/shallow` marker). The fingerprint feeds the cache key, so
/// the bump forces every stale entry to rebuild once instead of risking a
/// key computed under the old scheme.
pub const SCHEMA_VERSION: u32 = 2;

/// A single repo's cached graph layout along with the state that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutCacheEntry {
    /// Revision of the on-disk format; see [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// SHA-256 hex of `(repo_path, head_oid, sorted refs, variant)` — the
    /// identity of the repo state that produced the embedded [`GraphLayout`].
    pub cache_key: String,
    /// Absolute path to the repo whose layout is cached.
    pub repo_path: String,
    /// Layout-mode discriminator (e.g. `"fp=1"` for first-parent mode).
    /// Empty string for the default full-graph layout. Each variant gets its
    /// own cache file so toggling modes doesn't thrash a single entry.
    #[serde(default)]
    pub variant: String,
    /// HEAD OID at the time the layout was computed.
    pub head_oid: String,
    /// ISO-8601 timestamp of when the entry was written (informational only).
    pub generated_at: String,
    /// The computed layout that callers will reuse on a cache hit.
    pub layout: GraphLayout,
}

/// Borrowed twin of [`LayoutCacheEntry`] used to serialize a cache entry
/// *without cloning* the (potentially large) [`GraphLayout`].
///
/// The field set and names match [`LayoutCacheEntry`] exactly, so bytes
/// produced from this struct deserialize back into an owned
/// [`LayoutCacheEntry`] unchanged — serde matches by field name and JSON field
/// order is irrelevant on load. Use [`serialize_layout_entry`] to turn a borrow
/// into bytes, then [`write_layout_cache_bytes`] (typically off-thread) to
/// persist them.
#[derive(Debug, Serialize)]
pub struct LayoutCacheEntryRef<'a> {
    /// See [`LayoutCacheEntry::schema_version`].
    pub schema_version: u32,
    /// See [`LayoutCacheEntry::cache_key`].
    pub cache_key: &'a str,
    /// See [`LayoutCacheEntry::repo_path`].
    pub repo_path: &'a str,
    /// See [`LayoutCacheEntry::variant`].
    pub variant: &'a str,
    /// See [`LayoutCacheEntry::head_oid`].
    pub head_oid: &'a str,
    /// See [`LayoutCacheEntry::generated_at`].
    pub generated_at: &'a str,
    /// See [`LayoutCacheEntry::layout`].
    pub layout: &'a GraphLayout,
}

/// Serialize a borrowed cache entry to pretty JSON bytes, cloning nothing.
///
/// Callers hold the layout by reference (e.g. it lives in the active
/// `ProjectSlot`), so this avoids the deep `GraphLayout::clone` the owned
/// [`save_layout_cache`] path would otherwise force before serialization.
pub fn serialize_layout_entry(entry: &LayoutCacheEntryRef) -> std::io::Result<Vec<u8>> {
    serde_json::to_vec_pretty(entry).map_err(std::io::Error::other)
}

/// Write already-serialized cache bytes for `(repo_path, variant)`, creating
/// parent directories as needed.
///
/// Pairs with [`serialize_layout_entry`]: serialize once on the caller's thread
/// (from a borrow), then hand the owned bytes to this writer — cheap to move
/// into a `spawn_blocking` closure.
pub fn write_layout_cache_bytes(
    config_dir: &Path,
    repo_path: &str,
    variant: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    let path = layout_cache_path(config_dir, repo_path, variant);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

/// Compute the identity key for a repository's current state.
///
/// Combines the repo path, HEAD OID, the sorted list of `(ref_name, oid)`
/// pairs, and a layout-mode `variant` string into a single SHA-256 hex
/// string. Any change to HEAD, any ref create/delete, any ref movement, or
/// a different layout mode produces a different key.
///
/// An empty `variant` reproduces the pre-variant key material so existing
/// default-mode cache entries stay valid across upgrades.
pub fn compute_cache_key(
    repo_path: &str,
    head_oid: &str,
    refs: &[(String, String)],
    variant: &str,
) -> String {
    let mut sorted: Vec<&(String, String)> = refs.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let joined: String = sorted
        .iter()
        .map(|(n, o)| format!("{n}={o}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut material = format!("{repo_path}:{head_oid}:{joined}");
    if !variant.is_empty() {
        material.push(':');
        material.push_str(variant);
    }
    let mut hasher = Sha256::new();
    hasher.update(material.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Resolve the on-disk cache file path for a given repo + layout variant
/// under a config dir.
///
/// Mirrors the convention used by [`crate::project_cache`]: the caller passes
/// the BeardGit-specific config directory (usually `<os_config_dir>/beardgit`)
/// and this module appends its own `layouts/` subdirectory. Files live at
/// `<config_dir>/layouts/<sha256(repo_path[\n variant])>.json`. The default
/// (empty) variant hashes the repo path alone, matching the pre-variant
/// layout so existing cache files keep working.
pub fn layout_cache_path(config_dir: &Path, repo_path: &str, variant: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(repo_path.as_bytes());
    if !variant.is_empty() {
        hasher.update(b"\n");
        hasher.update(variant.as_bytes());
    }
    let name = format!("{:x}.json", hasher.finalize());
    config_dir.join("layouts").join(name)
}

/// Persist a [`LayoutCacheEntry`] to disk, creating parent directories as needed.
///
/// Overwrites any existing file for the same `(repo_path, variant)`. Returns
/// IO errors verbatim; callers typically log at `warn!` and discard on failure.
pub fn save_layout_cache(config_dir: &Path, entry: &LayoutCacheEntry) -> std::io::Result<()> {
    let path = layout_cache_path(config_dir, &entry.repo_path, &entry.variant);
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
    variant: &str,
) -> std::io::Result<Option<LayoutCacheEntry>> {
    let path = layout_cache_path(config_dir, repo_path, variant);
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
            variant: String::new(),
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
        let k1 = compute_cache_key("/repo", "aaaa", &refs, "");
        let k2 = compute_cache_key("/repo", "aaaa", &refs, "");
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 64, "sha256 hex is 64 chars");
    }

    #[test]
    fn compute_cache_key_changes_with_head() {
        let refs = vec![("refs/heads/main".to_string(), "aaaa".to_string())];
        assert_ne!(
            compute_cache_key("/repo", "aaaa", &refs, ""),
            compute_cache_key("/repo", "bbbb", &refs, ""),
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
            compute_cache_key("/repo", "aaaa", &r1, ""),
            compute_cache_key("/repo", "aaaa", &r2, ""),
        );
    }

    #[test]
    fn compute_cache_key_changes_with_variant() {
        let refs = vec![("refs/heads/main".to_string(), "aaaa".to_string())];
        assert_ne!(
            compute_cache_key("/repo", "aaaa", &refs, ""),
            compute_cache_key("/repo", "aaaa", &refs, "fp=1"),
        );
        assert_ne!(
            compute_cache_key("/repo", "aaaa", &refs, "fp=1"),
            compute_cache_key("/repo", "aaaa", &refs, "fp=0"),
        );
    }

    #[test]
    fn layout_cache_path_separates_variants() {
        let tmp = tempfile::tempdir().unwrap();
        let default_path = layout_cache_path(tmp.path(), "/repo", "");
        let fp_path = layout_cache_path(tmp.path(), "/repo", "fp=1");
        assert_ne!(
            default_path, fp_path,
            "each variant must get its own cache file"
        );
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let entry = sample_entry("/some/repo", "key-1");
        save_layout_cache(tmp.path(), &entry).unwrap();
        let loaded = load_layout_cache(tmp.path(), "/some/repo", "")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.cache_key, entry.cache_key);
        assert_eq!(loaded.repo_path, entry.repo_path);
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn save_and_load_round_trip_with_variant() {
        let tmp = tempfile::tempdir().unwrap();
        let mut entry = sample_entry("/some/repo", "key-fp");
        entry.variant = "fp=1".to_string();
        save_layout_cache(tmp.path(), &entry).unwrap();

        // The default-variant slot stays empty…
        assert!(
            load_layout_cache(tmp.path(), "/some/repo", "")
                .unwrap()
                .is_none()
        );
        // …while the variant slot round-trips.
        let loaded = load_layout_cache(tmp.path(), "/some/repo", "fp=1")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.cache_key, "key-fp");
        assert_eq!(loaded.variant, "fp=1");
    }

    #[test]
    fn load_returns_none_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let got = load_layout_cache(tmp.path(), "/none", "").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn load_returns_none_when_corrupt() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = "/repo";
        let path = layout_cache_path(tmp.path(), repo_path, "");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{not json").unwrap();
        let got = load_layout_cache(tmp.path(), repo_path, "").unwrap();
        assert!(
            got.is_none(),
            "corrupt file should be treated as miss, not panic/error"
        );
    }

    #[test]
    fn load_returns_none_when_schema_version_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_path = "/repo";
        let path = layout_cache_path(tmp.path(), repo_path, "");
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
        let got = load_layout_cache(tmp.path(), repo_path, "").unwrap();
        assert!(got.is_none());
    }
}
