//! Codex conversation transcript discovery.
//!
//! Codex persists each interactive / exec run as a JSONL rollout under
//! `~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl`. The
//! first line of every file is a `session_meta` event carrying the session
//! `id`, `cwd`, and its own RFC-3339 `timestamp`. This module walks the
//! rollout tree and emits one [`AiConversation`] per matching file so the
//! AI Sessions UI can list Codex conversations transcript-first — no
//! liveness PID file required.
//!
//! Sorting and discovery-window policy mirror the Claude Code
//! implementation so all providers feel uniform. Shared constants and the
//! `session_meta` deserialisation types live in [`super::sessions`].

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ai_provider::{AiConversation, AiError, AiProviderKind};

use crate::sessions::{DISCOVERY_WINDOW, SessionMeta, parse_rfc3339_to_unix_millis};

/// List Codex conversations whose rollout `cwd` matches `repo_path`.
///
/// Resolves `~/.codex/sessions/` and delegates to the internal
/// `list_conversations_in` core. Returns `Ok(vec![])` when `$HOME` can't be
/// resolved or the base directory doesn't exist yet — neither is an error
/// condition, it just means the user hasn't run Codex here.
pub fn list_conversations(repo_path: &Path) -> Result<Vec<AiConversation>, AiError> {
    let Some(home) = dirs::home_dir() else {
        return Ok(vec![]);
    };
    let base_dir = home.join(".codex/sessions");
    list_conversations_in(&base_dir, repo_path)
}

/// Testable core of [`list_conversations`]: walks `base_dir` directly
/// without touching the real `$HOME`.
///
/// See [`list_conversations`] for the behavioural contract. The split
/// exists solely so unit tests can stage rollouts under a
/// `tempfile::TempDir` without monkey-patching `dirs::home_dir`.
pub(crate) fn list_conversations_in(
    base_dir: &Path,
    repo_path: &Path,
) -> Result<Vec<AiConversation>, AiError> {
    if !base_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut out: Vec<AiConversation> = Vec::new();
    collect_conversations(base_dir, repo_path, &mut out);

    // Most-recent activity first — matches the UX expectation of the
    // sessions sidebar and mirrors the Claude Code implementation.
    out.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
    Ok(out)
}

/// Recursively descend into `dir`, appending one [`AiConversation`] per
/// readable `*.jsonl` rollout whose `cwd` is `repo_path` (or a subpath).
///
/// Malformed / unparseable files are silently skipped rather than
/// propagating errors — a single corrupt rollout must not poison the
/// whole listing.
fn collect_conversations(dir: &Path, repo_path: &Path, out: &mut Vec<AiConversation>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_conversations(&path, repo_path, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        if let Some(convo) = parse_rollout(&path, repo_path) {
            out.push(convo);
        }
    }
}

/// Parse a single rollout JSONL file into an [`AiConversation`].
///
/// Returns `None` when any of the following hold:
/// * metadata read / mtime resolution fails,
/// * the rollout's mtime is older than [`DISCOVERY_WINDOW`],
/// * the file is empty or its first line isn't a `session_meta` record,
/// * the payload `cwd` is missing or does not match `repo_path`.
///
/// Trailing slashes on `cwd` are normalised before comparison so that a
/// rollout recorded at `/repo/` still matches a `repo_path` of `/repo`.
fn parse_rollout(path: &Path, repo_path: &Path) -> Option<AiConversation> {
    // Cheap `stat` first — parsing is an order of magnitude more work.
    let modified = path.metadata().and_then(|m| m.modified()).ok()?;
    if SystemTime::now()
        .duration_since(modified)
        .map(|age| age > DISCOVERY_WINDOW)
        .unwrap_or(false)
    {
        return None;
    }
    let last_activity_at = modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;

    // First-line only — Codex's session_meta is always the first record.
    let contents = fs::read_to_string(path).ok()?;
    let first_line = contents.lines().next()?;
    let meta: SessionMeta = serde_json::from_str(first_line).ok()?;
    if meta.kind != "session_meta" {
        return None;
    }

    let payload_cwd = meta.payload.cwd.as_deref().map(PathBuf::from)?;
    if !cwd_matches(&payload_cwd, repo_path) {
        return None;
    }

    let created_at = meta
        .payload
        .timestamp
        .as_deref()
        .and_then(parse_rfc3339_to_unix_millis)
        .unwrap_or(last_activity_at);

    Some(AiConversation {
        id: meta.payload.id,
        provider: AiProviderKind::Codex,
        cwd: payload_cwd,
        created_at,
        last_activity_at,
        // TODO(phase-7+): Codex rollouts store the first user prompt on
        // later lines under varying shapes. Extracting it reliably is out
        // of scope for this MVP slice — leave the hook in place.
        title: String::new(),
        // Codex does not fork conversations the way Claude's `--resume`
        // does; there is no parent pointer to surface.
        parent_id: None,
    })
}

/// True when `candidate` equals `repo` or sits beneath it.
///
/// Both sides get their trailing `/` stripped (except for the bare root)
/// before comparison so we're robust against Codex recording a path as
/// `/Users/x/repo/` while the UI asks about `/Users/x/repo`.
fn cwd_matches(candidate: &Path, repo: &Path) -> bool {
    let c = normalise(candidate);
    let r = normalise(repo);
    c == r || c.starts_with(&r)
}

/// Strip a single trailing `/` off a path string, preserving bare root.
///
/// Used for byte-level trailing-slash normalisation before the `cwd`
/// comparison; not a full canonicalisation — we must not hit the
/// filesystem here because the paths may no longer exist.
fn normalise(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if s.len() > 1 {
        PathBuf::from(s.trim_end_matches('/'))
    } else {
        PathBuf::from(s.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    /// Write a rollout file containing a single `session_meta` first line
    /// with the provided fields.
    fn write_meta_rollout(dir: &Path, rel: &str, id: &str, cwd: &str, ts: &str) -> PathBuf {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let line = format!(
            r#"{{"timestamp":"{ts}","type":"session_meta","payload":{{"id":"{id}","timestamp":"{ts}","cwd":"{cwd}","source":"exec","originator":"codex_exec","cli_version":"0.121.0"}}}}
"#
        );
        fs::write(&path, line).unwrap();
        path
    }

    /// Write a rollout whose first line is *not* a `session_meta` record.
    fn write_other_first_line(dir: &Path, rel: &str) -> PathBuf {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"type":"event_msg","payload":{"type":"task_started"}}
"#,
        )
        .unwrap();
        path
    }

    /// Write a rollout whose first line is invalid JSON.
    fn write_malformed(dir: &Path, rel: &str) -> PathBuf {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json at all\n").unwrap();
        path
    }

    #[test]
    fn lists_rollouts_matching_cwd() {
        let dir = tempfile::tempdir().unwrap();
        write_meta_rollout(
            dir.path(),
            "2026/04/20/rollout-match.jsonl",
            "match-id",
            "/my/repo",
            "2026-04-20T10:00:00.000Z",
        );
        write_meta_rollout(
            dir.path(),
            "2026/04/20/rollout-other.jsonl",
            "other-id",
            "/some/other/path",
            "2026-04-20T10:01:00.000Z",
        );

        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.id, "match-id");
        assert_eq!(c.cwd, Path::new("/my/repo"));
        assert_eq!(c.provider, AiProviderKind::Codex);
        assert_eq!(c.title, "");
        assert!(c.parent_id.is_none());
        // Created_at should come from the embedded RFC-3339 timestamp.
        let expected = parse_rfc3339_to_unix_millis("2026-04-20T10:00:00.000Z").unwrap();
        assert_eq!(c.created_at, expected);
    }

    #[test]
    fn skips_non_session_meta_first_line() {
        let dir = tempfile::tempdir().unwrap();
        write_other_first_line(dir.path(), "2026/04/20/rollout-bad.jsonl");
        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn skips_malformed_first_line() {
        let dir = tempfile::tempdir().unwrap();
        write_malformed(dir.path(), "2026/04/20/rollout-garbage.jsonl");
        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn sorts_desc_by_mtime() {
        let dir = tempfile::tempdir().unwrap();
        let older = write_meta_rollout(
            dir.path(),
            "2026/04/19/rollout-older.jsonl",
            "older-id",
            "/my/repo",
            "2026-04-19T10:00:00.000Z",
        );
        let newer = write_meta_rollout(
            dir.path(),
            "2026/04/20/rollout-newer.jsonl",
            "newer-id",
            "/my/repo",
            "2026-04-20T10:00:00.000Z",
        );

        // Force a known mtime ordering.
        set_file_mtime_seconds_ago(&older, 1000);
        set_file_mtime_seconds_ago(&newer, 0);

        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "newer-id");
        assert_eq!(result[1].id, "older-id");
        assert!(result[0].last_activity_at >= result[1].last_activity_at);
    }

    #[test]
    fn empty_when_base_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let result = list_conversations_in(&missing, Path::new("/my/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn matches_subdirectory_cwd() {
        let dir = tempfile::tempdir().unwrap();
        write_meta_rollout(
            dir.path(),
            "2026/04/20/rollout-wt.jsonl",
            "wt-id",
            "/repo/worktree-1",
            "2026-04-20T10:00:00.000Z",
        );
        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].cwd, Path::new("/repo/worktree-1"));
    }

    #[test]
    fn trailing_slash_normalised_on_comparison() {
        let dir = tempfile::tempdir().unwrap();
        write_meta_rollout(
            dir.path(),
            "2026/04/20/rollout-trail.jsonl",
            "trail-id",
            "/my/repo/",
            "2026-04-20T10:00:00.000Z",
        );
        // Caller path has no trailing slash — must still match.
        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "trail-id");
    }

    #[test]
    fn skips_stale_rollouts_outside_discovery_window() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_meta_rollout(
            dir.path(),
            "2026/01/01/rollout-stale.jsonl",
            "stale-id",
            "/my/repo",
            "2026-01-01T10:00:00.000Z",
        );
        // 40 days ago — well outside the 30-day discovery window.
        set_file_mtime_seconds_ago(&path, 40 * 24 * 60 * 60);
        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert!(result.is_empty(), "stale rollout should be skipped");
    }

    // ─── unix mtime helper for tests ───

    /// Set `path`'s atime+mtime to `N` seconds before now.
    ///
    /// Mirrors the helper used in Claude's `conversations.rs` tests — we
    /// lean on `libc::utimes` directly so the workspace doesn't need a
    /// `filetime` dev-dep just for two tests.
    #[cfg(unix)]
    fn set_file_mtime_seconds_ago(path: &Path, seconds: u64) {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let target = SystemTime::now() - Duration::from_secs(seconds);
        let since_epoch = target.duration_since(UNIX_EPOCH).unwrap();
        let sec = since_epoch.as_secs() as libc::time_t;
        let usec = (since_epoch.subsec_micros()) as libc::suseconds_t;

        let times = [
            libc::timeval {
                tv_sec: sec,
                tv_usec: usec,
            },
            libc::timeval {
                tv_sec: sec,
                tv_usec: usec,
            },
        ];
        let cpath = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: `cpath` is a valid NUL-terminated C string and `times`
        // points at a 2-element array matching the utimes(2) ABI.
        let rc = unsafe { libc::utimes(cpath.as_ptr(), times.as_ptr()) };
        assert_eq!(rc, 0, "utimes failed for {}", path.display());
    }

    #[cfg(not(unix))]
    fn set_file_mtime_seconds_ago(_path: &Path, _seconds: u64) {
        // Non-unix CI would need a different mechanism; tests that rely
        // on this helper currently run on macOS/Linux only.
    }
}
