//! Codex session file discovery + parsing.
//!
//! Codex persists each exec / interactive run as a JSONL file under
//! `~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl`. The first
//! line of every file is a `session_meta` event carrying session-level
//! metadata; subsequent lines are timeline events (turn contexts, response
//! items, tool calls, etc.). We only need the first line to populate an
//! [`AiSession`].
//!
//! **Probe verified on 2026-04-20** against `codex-cli 0.121.0`. A real
//! invocation (`codex exec "print hello"` in `/tmp/codex-test`) produced the
//! following first-line shape (trimmed):
//!
//! ```json
//! {
//!   "timestamp": "2026-04-20T21:58:55.163Z",
//!   "type": "session_meta",
//!   "payload": {
//!     "id": "019dace7-5260-7762-a330-072ff38df69f",
//!     "timestamp": "2026-04-20T21:58:54.320Z",
//!     "cwd": "/private/tmp/codex-test",
//!     "originator": "codex_exec",
//!     "cli_version": "0.121.0",
//!     "source": "exec",
//!     "model_provider": "openai"
//!   }
//! }
//! ```
//!
//! Codex does **not** record a PID in the session metadata, so liveness is
//! inferred from the session file's last-modified timestamp (see
//! [`is_file_active`]).
//!
//! `codex` itself does not honor a `CODEX_HOME` env var, so we accept a
//! `base_dir: &Path` argument for testability — callers pass
//! `~/.codex/sessions` in production and a tempdir in tests.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ai_provider::{AiProviderKind, AiSession, SessionKind};
use serde::Deserialize;

/// How stale a session file may be before it's considered inactive.
///
/// Codex doesn't surface liveness directly, so we fall back to the mtime
/// heuristic from the plan: files touched within this window are treated as
/// live.
pub const ACTIVE_WINDOW: Duration = Duration::from_secs(5 * 60);

/// First-line JSON shape — only the fields we read.
#[derive(Debug, Deserialize)]
struct SessionMeta {
    #[serde(rename = "type")]
    kind: String,
    payload: SessionMetaPayload,
}

#[derive(Debug, Deserialize)]
struct SessionMetaPayload {
    id: String,
    /// ISO 8601 timestamp — e.g. `"2026-04-20T21:58:54.320Z"`.
    timestamp: Option<String>,
    cwd: Option<String>,
    /// `"exec"` → headless, anything else (interactive, tui, …) → interactive.
    source: Option<String>,
}

/// Walk `base_dir` (expected shape: `YYYY/MM/DD/rollout-*.jsonl`) and return
/// every parsable session.
///
/// Unparseable or truncated files are silently skipped — matches the
/// Claude Code implementation's tolerance for partial state.
pub(crate) fn load_sessions(base_dir: &Path) -> Vec<AiSession> {
    let mut sessions = Vec::new();
    collect_jsonl(base_dir, &mut sessions);
    sessions
}

/// Recursively descend into `dir`, accumulating one [`AiSession`] per
/// readable `*.jsonl` file.
fn collect_jsonl(dir: &Path, out: &mut Vec<AiSession>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_jsonl(&path, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        if let Some(session) = parse_session_file(&path) {
            out.push(session);
        }
    }
}

/// Parse the first line of `path` into an [`AiSession`], or return `None`
/// if the file is empty / malformed / not a `session_meta` record.
fn parse_session_file(path: &Path) -> Option<AiSession> {
    let contents = fs::read_to_string(path).ok()?;
    let first_line = contents.lines().next()?;
    let meta: SessionMeta = serde_json::from_str(first_line).ok()?;
    if meta.kind != "session_meta" {
        return None;
    }

    let cwd = meta
        .payload
        .cwd
        .as_deref()
        .map(Path::new)
        .unwrap_or(path)
        .to_path_buf();

    let started_at = meta
        .payload
        .timestamp
        .as_deref()
        .and_then(parse_rfc3339_to_unix_millis);

    let kind = match meta.payload.source.as_deref() {
        Some("exec") => SessionKind::Headless,
        _ => SessionKind::Interactive,
    };

    let is_active = path
        .metadata()
        .and_then(|m| m.modified())
        .map(is_file_active)
        .unwrap_or(false);

    Some(AiSession {
        id: meta.payload.id,
        provider: AiProviderKind::Codex,
        cwd,
        started_at,
        kind,
        is_active,
        worktree_path: None,
        background_status: None,
        task_id: None,
    })
}

/// Very small RFC-3339 parser — avoids pulling `chrono` just to convert the
/// timestamp to Unix milliseconds. Returns `None` on any parsing failure.
fn parse_rfc3339_to_unix_millis(ts: &str) -> Option<u64> {
    // Expected: `YYYY-MM-DDTHH:MM:SS[.fff]Z`
    let (date, rest) = ts.split_once('T')?;
    let time = rest.strip_suffix('Z').unwrap_or(rest);
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;

    let (hms, frac) = match time.split_once('.') {
        Some((hms, frac)) => (hms, frac),
        None => (time, "0"),
    };
    let mut hms_parts = hms.split(':');
    let hour: u32 = hms_parts.next()?.parse().ok()?;
    let minute: u32 = hms_parts.next()?.parse().ok()?;
    let second: u32 = hms_parts.next()?.parse().ok()?;

    // Civil → Julian day number (Howard Hinnant's algorithm).
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days_since_epoch = era * 146_097 + doe as i64 - 719_468;

    let secs = days_since_epoch * 86_400
        + i64::from(hour) * 3_600
        + i64::from(minute) * 60
        + i64::from(second);
    if secs < 0 {
        return None;
    }
    let millis_frac: u64 = {
        // Use up to three fractional digits.
        let mut buf = String::from(frac);
        buf.truncate(3);
        while buf.len() < 3 {
            buf.push('0');
        }
        buf.parse().ok()?
    };
    Some(secs as u64 * 1_000 + millis_frac)
}

/// True when `modified` is within [`ACTIVE_WINDOW`] of now.
pub(crate) fn is_file_active(modified: SystemTime) -> bool {
    let now = SystemTime::now();
    match now.duration_since(modified) {
        Ok(age) => age <= ACTIVE_WINDOW,
        // mtime in the future — treat as active (clock skew).
        Err(_) => true,
    }
}

/// Re-check liveness for an already-loaded [`AiSession`] by scanning the
/// session directory again and matching on id.
pub(crate) fn is_session_active(base_dir: &Path, session_id: &str) -> bool {
    let mut found = false;
    check_session_active(base_dir, session_id, &mut found);
    found
}

fn check_session_active(dir: &Path, session_id: &str, out: &mut bool) {
    if *out {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if *out {
            return;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            check_session_active(&path, session_id, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        // Fast-path: filename embeds the session UUID.
        let name_ok = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.contains(session_id))
            .unwrap_or(false);
        if !name_ok {
            // Fall back to parsing the first line.
            match parse_session_file(&path) {
                Some(s) if s.id == session_id => {}
                _ => continue,
            }
        }
        if let Ok(meta) = path.metadata()
            && let Ok(modified) = meta.modified()
        {
            *out = is_file_active(modified);
            return;
        }
    }
}

/// Helper used only by tests and a liveness fallback path that needs the
/// current system time as millis-since-epoch.
#[allow(dead_code)]
pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_meta(dir: &Path, rel: &str, id: &str, cwd: &str, source: &str, ts: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let line = format!(
            r#"{{"timestamp":"{ts}","type":"session_meta","payload":{{"id":"{id}","timestamp":"{ts}","cwd":"{cwd}","source":"{source}","originator":"codex_exec","cli_version":"0.121.0"}}}}
"#
        );
        fs::write(path, line).unwrap();
    }

    #[test]
    fn loads_single_session() {
        let dir = tempfile::tempdir().unwrap();
        write_meta(
            dir.path(),
            "2026/04/20/rollout-2026-04-20T21-58-54-abc.jsonl",
            "abc-uuid",
            "/tmp/repo",
            "exec",
            "2026-04-20T21:58:54.320Z",
        );

        let sessions = load_sessions(dir.path());
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.id, "abc-uuid");
        assert_eq!(s.cwd, Path::new("/tmp/repo"));
        assert_eq!(s.kind, SessionKind::Headless);
        assert_eq!(s.provider, AiProviderKind::Codex);
        assert!(s.started_at.is_some());
    }

    #[test]
    fn interactive_source_classified_as_interactive() {
        let dir = tempfile::tempdir().unwrap();
        write_meta(
            dir.path(),
            "2026/04/20/rollout-tui.jsonl",
            "tui-uuid",
            "/tmp/repo",
            "tui",
            "2026-04-20T12:00:00.000Z",
        );
        let sessions = load_sessions(dir.path());
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].kind, SessionKind::Interactive);
    }

    #[test]
    fn ignores_malformed_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("2026/04/20/rollout-bad.jsonl");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not-json\n").unwrap();

        let sessions = load_sessions(dir.path());
        assert!(sessions.is_empty());
    }

    #[test]
    fn ignores_non_session_meta_first_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("2026/04/20/rollout-other.jsonl");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"type":"event_msg","payload":{"type":"task_started"}}
"#,
        )
        .unwrap();
        assert!(load_sessions(dir.path()).is_empty());
    }

    #[test]
    fn ignores_non_jsonl_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("2026/04/20/README.md");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "hello").unwrap();
        assert!(load_sessions(dir.path()).is_empty());
    }

    #[test]
    fn loads_multiple_sessions_across_date_dirs() {
        let dir = tempfile::tempdir().unwrap();
        write_meta(
            dir.path(),
            "2026/04/19/rollout-a.jsonl",
            "id-a",
            "/tmp/a",
            "exec",
            "2026-04-19T10:00:00.000Z",
        );
        write_meta(
            dir.path(),
            "2026/04/20/rollout-b.jsonl",
            "id-b",
            "/tmp/b",
            "tui",
            "2026-04-20T10:00:00.000Z",
        );

        let mut sessions = load_sessions(dir.path());
        sessions.sort_by(|x, y| x.id.cmp(&y.id));
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "id-a");
        assert_eq!(sessions[1].id, "id-b");
    }

    #[test]
    fn missing_dir_returns_empty() {
        let sessions = load_sessions(Path::new("/definitely/does/not/exist/codex"));
        assert!(sessions.is_empty());
    }

    #[test]
    fn rfc3339_parses_typical_value() {
        let millis = parse_rfc3339_to_unix_millis("2026-04-20T21:58:54.320Z").unwrap();
        // 2026-04-20T21:58:54.320Z = 1776722334320ms
        assert_eq!(millis, 1_776_722_334_320);
    }

    #[test]
    fn rfc3339_rejects_garbage() {
        assert!(parse_rfc3339_to_unix_millis("nope").is_none());
    }

    #[test]
    fn is_file_active_recent_true() {
        let now = SystemTime::now();
        assert!(is_file_active(now));
    }

    #[test]
    fn is_file_active_old_false() {
        let old = SystemTime::now() - Duration::from_secs(60 * 60);
        assert!(!is_file_active(old));
    }

    #[test]
    fn loaded_session_is_active_when_file_is_fresh() {
        let dir = tempfile::tempdir().unwrap();
        write_meta(
            dir.path(),
            "2026/04/20/rollout-fresh.jsonl",
            "fresh-id",
            "/tmp/repo",
            "exec",
            "2026-04-20T21:58:54.320Z",
        );
        let sessions = load_sessions(dir.path());
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].is_active, "fresh file must be active");
    }

    #[test]
    fn is_session_active_finds_fresh_session_by_id() {
        let dir = tempfile::tempdir().unwrap();
        write_meta(
            dir.path(),
            "2026/04/20/rollout-fresh.jsonl",
            "fresh-id",
            "/tmp/repo",
            "exec",
            "2026-04-20T21:58:54.320Z",
        );
        // Even if we pretend the AiSession's cached flag is stale, the
        // directory walk should still pick up the fresh mtime.
        assert!(is_session_active(dir.path(), "fresh-id"));
        assert!(!is_session_active(dir.path(), "no-such-id"));
    }

    #[test]
    fn is_session_active_stale_file_is_inactive() {
        let dir = tempfile::tempdir().unwrap();
        let rel = "2026/04/20/rollout-stale.jsonl";
        write_meta(
            dir.path(),
            rel,
            "stale-id",
            "/tmp/repo",
            "exec",
            "2026-04-20T21:58:54.320Z",
        );
        // Force mtime an hour in the past.
        let path = dir.path().join(rel);
        let old = SystemTime::now() - Duration::from_secs(60 * 60);
        // `filetime` would be nicer but avoid adding a dep for one test:
        // use libc on unix, fall back to skipping the assertion elsewhere.
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let cpath = std::ffi::CString::new(path.to_string_lossy().as_bytes()).unwrap();
            // Seconds since UNIX epoch for the desired mtime.
            let secs = old.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let atime = libc::timeval {
                tv_sec: secs as libc::time_t,
                tv_usec: 0,
            };
            let mtime = atime;
            let times = [atime, mtime];
            let rc = unsafe { libc::utimes(cpath.as_ptr(), times.as_ptr()) };
            assert_eq!(rc, 0, "utimes failed");
            // Sanity: metadata actually moved.
            let meta = std::fs::metadata(&path).unwrap();
            assert!(meta.mtime() < (now_millis() / 1_000) as i64);
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            let _ = old;
            return; // Skip on non-unix — no portable way to backdate mtime.
        }
        #[cfg(unix)]
        assert!(!is_session_active(dir.path(), "stale-id"));
    }
}
