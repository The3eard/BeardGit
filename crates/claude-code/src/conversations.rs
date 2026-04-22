//! Claude Code conversation transcript discovery.
//!
//! Claude Code persists every conversation as a JSONL file under
//! `~/.claude/projects/{cwd-slug}/{uuid}.jsonl` where each line is one
//! structured record (`user`, `assistant`, hook attachments, etc.). This
//! module walks that directory for a given repo path and surfaces a list of
//! [`AiConversation`] metadata rows suitable for the AI Sessions UI.
//!
//! Transcript-first: a conversation row is emitted whether or not a live
//! `claude` process is still attached to it.

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ai_provider::{AiConversation, AiError, AiProviderKind};
use chrono::DateTime;
use serde_json::Value;

/// Transcripts older than this are skipped during the initial directory
/// walk.
///
/// Claude Code never prunes `~/.claude/projects/{slug}/*.jsonl`, so repos
/// the user has touched for years accumulate hundreds of stale transcripts
/// that add noise to the AI Sessions view and slow its first paint. 30 days
/// of retention mirrors the Codex walker's window and covers typical
/// "show me last month's runs" UX without scanning the full history.
const DISCOVERY_WINDOW: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Maximum character length of the extracted conversation title.
///
/// The UI renders titles in a single line; long prompts get truncated with
/// an ellipsis upstream. 80 chars is enough signal for the user to tell
/// two conversations apart without dominating the row.
const TITLE_MAX_CHARS: usize = 80;

/// Compute the Claude Code project-slug directory name for a working dir.
///
/// Claude stores transcripts under `~/.claude/projects/{slug}/` where the
/// slug is produced by prefixing `-` and substituting every `/` for `-`.
/// No other characters are rewritten — spaces, dots, and underscores in the
/// original path survive as-is. Confirmed against a user's disk on
/// 2026-04-22 (`/Users/adolfo/Projects/BeardGit` → `-Users-adolfo-Projects-BeardGit`).
pub(crate) fn cwd_to_slug(cwd: &Path) -> String {
    let raw = cwd.to_string_lossy();
    // Strip a single trailing `/` so `/Users/x/` and `/Users/x` slug the
    // same way. Bare `/` is special — it must survive as the lone prefix
    // dash (`-`), not become an empty string.
    let trimmed = if raw.len() > 1 {
        raw.trim_end_matches('/')
    } else {
        &raw
    };
    trimmed.replace('/', "-")
}

/// List conversation transcripts from the Claude Code project store that
/// were opened for `repo_path`.
///
/// Resolves `~/.claude/projects/{cwd_to_slug(repo_path)}/` and delegates
/// the heavy lifting to [`list_conversations_in`]. Returns `Ok(vec![])`
/// when the user has no home directory or the slug dir doesn't exist —
/// neither is an error condition, just "no transcripts yet".
pub fn list_conversations(repo_path: &Path) -> Result<Vec<AiConversation>, AiError> {
    let Some(home) = dirs::home_dir() else {
        return Ok(vec![]);
    };
    let slug_dir = home.join(".claude/projects").join(cwd_to_slug(repo_path));
    list_conversations_in(&slug_dir, repo_path)
}

/// Testable core of [`list_conversations`]: walks `slug_dir` directly
/// without touching the real `$HOME`.
///
/// See [`list_conversations`] for the behavioural contract. The split
/// exists solely so unit tests can stage `*.jsonl` fixtures under a
/// `tempfile::TempDir` without monkey-patching `dirs::home_dir`.
pub(crate) fn list_conversations_in(
    slug_dir: &Path,
    repo_path: &Path,
) -> Result<Vec<AiConversation>, AiError> {
    if !slug_dir.is_dir() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(slug_dir)?;
    let now = SystemTime::now();
    let mut out: Vec<AiConversation> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();

        // Claude creates a sibling subdir per transcript (e.g. `{uuid}/` for
        // sub-agents, and the `memory/` summariser store). Both show up as
        // directory entries in `read_dir` — the `.jsonl` extension filter
        // already excludes them, but we keep the explicit `memory` guard
        // as defence-in-depth matching the task spec.
        if path.file_name().and_then(|n| n.to_str()) == Some("memory") {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }

        // Cheap `stat` before the read — transcripts older than the
        // discovery window aren't worth parsing.
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified = match meta.modified().or_else(|_| meta.created()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if now
            .duration_since(modified)
            .map(|age| age > DISCOVERY_WINDOW)
            .unwrap_or(false)
        {
            continue;
        }

        let last_activity_at = match modified.duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis() as u64,
            Err(_) => continue,
        };

        let id = match path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => stem.to_string(),
            None => continue,
        };

        let Some(convo) = build_conversation(&path, id, repo_path, last_activity_at) else {
            continue;
        };
        out.push(convo);
    }

    // Most-recent activity first — matches the UX expectation of the
    // sessions sidebar.
    out.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
    Ok(out)
}

/// Parse a single `.jsonl` transcript into an [`AiConversation`].
///
/// Returns `None` when the file contains zero parseable records (fully
/// empty, all-garbage, or only contains attachment records with no
/// accompanying content) — the caller skips such files entirely.
fn build_conversation(
    path: &Path,
    id: String,
    repo_path: &Path,
    last_activity_at: u64,
) -> Option<AiConversation> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut first_meta: Option<FirstRecordMeta> = None;
    let mut title: Option<String> = None;

    for line in reader.lines() {
        let Ok(line) = line else { break };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };

        if first_meta.is_none() {
            first_meta = Some(extract_first_meta(&record));
        }

        if title.is_none()
            && let Some(candidate) = extract_title_candidate(&record)
        {
            title = Some(candidate);
        }

        // Early-exit once we have everything: picking out the first real
        // prompt is all we need, and transcripts can be huge (MBs).
        if first_meta.is_some() && title.is_some() {
            break;
        }
    }

    let meta = first_meta?; // File had zero parseable records → skip.
    let created_at = meta.timestamp_ms.unwrap_or(last_activity_at);

    Some(AiConversation {
        id,
        provider: AiProviderKind::ClaudeCode,
        cwd: repo_path.to_path_buf(),
        created_at,
        last_activity_at,
        title: title.unwrap_or_default(),
        parent_id: meta.parent_id,
    })
}

/// Metadata pulled off the first parseable record in a transcript.
///
/// The first record is the only one we inspect for fork / timestamp
/// signals — `parentUuid` on any later record refers to *intra-transcript*
/// chaining, not a fork relationship.
#[derive(Debug, Default)]
struct FirstRecordMeta {
    /// First 8 chars of a non-null `parentUuid` on the first record —
    /// signals this transcript was forked from another conversation.
    parent_id: Option<String>,
    /// First record's own `timestamp` parsed as unix ms. `None` when the
    /// field is missing or unparseable; the caller falls back to `mtime`.
    timestamp_ms: Option<u64>,
}

/// Build [`FirstRecordMeta`] from the first parseable JSON record in a
/// transcript. Never fails — missing / malformed fields simply leave the
/// corresponding slot at `None`.
fn extract_first_meta(record: &Value) -> FirstRecordMeta {
    let parent_id = record
        .get("parentUuid")
        .and_then(|v| v.as_str())
        .map(|s| s.chars().take(8).collect::<String>())
        .filter(|s| !s.is_empty());

    let timestamp_ms = record
        .get("timestamp")
        .and_then(|v| v.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp_millis() as u64);

    FirstRecordMeta {
        parent_id,
        timestamp_ms,
    }
}

/// Extract a title candidate from a transcript record, if any.
///
/// Returns `None` when the record is:
/// * not a `user` entry,
/// * a meta entry (`isMeta == true`),
/// * a slash-command envelope (`<command-name>…`, `<command-message>…`,
///   `<local-command-caveat>…`),
/// * a content-less record (no `message.content` or all non-text parts),
/// * or an entry whose extracted text is empty after trimming.
///
/// Otherwise returns the cleaned title: internal newlines collapsed to
/// single spaces and truncated at [`TITLE_MAX_CHARS`] using char-boundary
/// safe iteration.
fn extract_title_candidate(record: &Value) -> Option<String> {
    if record.get("type").and_then(|v| v.as_str()) != Some("user") {
        return None;
    }
    if record.get("isMeta").and_then(|v| v.as_bool()) == Some(true) {
        return None;
    }

    let content = record.get("message")?.get("content")?;
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => {
            let joined: Vec<&str> = parts
                .iter()
                .filter(|p| p.get("type").and_then(|v| v.as_str()) == Some("text"))
                .filter_map(|p| p.get("text").and_then(|v| v.as_str()))
                .collect();
            joined.join(" ")
        }
        _ => return None,
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("<command-name>")
        || trimmed.starts_with("<command-message>")
        || trimmed.starts_with("<local-command-caveat>")
    {
        return None;
    }

    // Collapse every run of whitespace — including `\n`, `\r`, `\t` — down
    // to a single space so the title renders as one line. `split_whitespace`
    // ignores empty segments, which also eats leading/trailing padding.
    let collapsed = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");

    // Char-boundary safe truncation. `str::truncate` would panic on a
    // multibyte split — `chars().take(N)` always lands on a boundary.
    let truncated: String = collapsed.chars().take(TITLE_MAX_CHARS).collect();
    let final_title = truncated.trim_end().to_string();

    if final_title.is_empty() {
        None
    } else {
        Some(final_title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // ─── cwd_to_slug ───

    #[test]
    fn cwd_to_slug_encodes_macos_path() {
        assert_eq!(
            cwd_to_slug(Path::new("/Users/x/Projects/Y")),
            "-Users-x-Projects-Y"
        );
    }

    #[test]
    fn cwd_to_slug_preserves_spaces() {
        assert_eq!(cwd_to_slug(Path::new("/tmp/foo bar")), "-tmp-foo bar");
    }

    #[test]
    fn cwd_to_slug_strips_trailing_slash() {
        assert_eq!(cwd_to_slug(Path::new("/Users/x/")), "-Users-x");
        assert_eq!(cwd_to_slug(Path::new("/Users/x")), "-Users-x");
    }

    #[test]
    fn cwd_to_slug_handles_root() {
        assert_eq!(cwd_to_slug(Path::new("/")), "-");
    }

    // ─── helpers ───

    fn write_jsonl(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    fn make_user_line(text: &str, ts: &str, parent_uuid: Option<&str>) -> String {
        let parent = match parent_uuid {
            Some(u) => format!("\"{u}\""),
            None => "null".to_string(),
        };
        format!(
            "{{\"parentUuid\":{parent},\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":{text_json}}},\"timestamp\":\"{ts}\"}}",
            text_json = serde_json::Value::String(text.to_string())
        )
    }

    // ─── list_conversations_in ───

    #[test]
    fn list_conversations_finds_recent_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        let line = make_user_line("hello world", "2026-04-22T10:00:00.000Z", None);
        write_jsonl(dir.path(), "abc-123.jsonl", &[&line]);

        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.id, "abc-123");
        assert_eq!(c.title, "hello world");
        assert_eq!(c.provider, AiProviderKind::ClaudeCode);
        assert_eq!(c.cwd, Path::new("/my/repo"));
        assert!(c.parent_id.is_none());
        // created_at matches the embedded timestamp (2026-04-22T10:00:00Z).
        let expected = DateTime::parse_from_rfc3339("2026-04-22T10:00:00.000Z")
            .unwrap()
            .timestamp_millis() as u64;
        assert_eq!(c.created_at, expected);
    }

    #[test]
    fn list_conversations_sorts_by_mtime_desc() {
        let dir = tempfile::tempdir().unwrap();
        let older = make_user_line("older", "2026-04-20T10:00:00.000Z", None);
        let newer = make_user_line("newer", "2026-04-22T10:00:00.000Z", None);
        let older_path = write_jsonl(dir.path(), "older.jsonl", &[&older]);
        let newer_path = write_jsonl(dir.path(), "newer.jsonl", &[&newer]);

        // Force a known mtime ordering: older gets mtime = now - 1000s,
        // newer gets mtime = now. Uses `libc::utimes` directly (no
        // `filetime` dep in the workspace).
        set_file_mtime_seconds_ago(&older_path, 1000);
        set_file_mtime_seconds_ago(&newer_path, 0);

        let result = list_conversations_in(dir.path(), Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "newer");
        assert_eq!(result[1].id, "older");
        assert!(result[0].last_activity_at >= result[1].last_activity_at);
    }

    #[test]
    fn list_conversations_extracts_fork_parent() {
        let dir = tempfile::tempdir().unwrap();
        let line = make_user_line(
            "forked",
            "2026-04-22T10:00:00.000Z",
            Some("1b6423b0-b6bc-4fe5-9f07-93b472d7aa47"),
        );
        write_jsonl(dir.path(), "fork.jsonl", &[&line]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].parent_id.as_deref(), Some("1b6423b0"));
    }

    #[test]
    fn list_conversations_truncates_long_title() {
        let dir = tempfile::tempdir().unwrap();
        // Prompt with >80 chars and embedded newlines.
        let prompt = "line one\nline two has quite a lot of words and extends well past the eighty character budget we allow";
        let line = make_user_line(prompt, "2026-04-22T10:00:00.000Z", None);
        write_jsonl(dir.path(), "long.jsonl", &[&line]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        let title = &result[0].title;
        assert!(!title.contains('\n'), "title must not contain newlines");
        assert!(
            title.chars().count() <= TITLE_MAX_CHARS,
            "title too long: {} chars",
            title.chars().count()
        );
        assert!(title.starts_with("line one line two"));
    }

    #[test]
    fn list_conversations_respects_discovery_window() {
        let dir = tempfile::tempdir().unwrap();
        let line = make_user_line("stale", "2020-01-01T10:00:00.000Z", None);
        let path = write_jsonl(dir.path(), "stale.jsonl", &[&line]);
        // 40 days ago — well outside the 30-day window.
        set_file_mtime_seconds_ago(&path, 40 * 24 * 60 * 60);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert!(result.is_empty(), "stale transcript should be skipped");
    }

    #[test]
    fn list_conversations_handles_malformed_first_line() {
        let dir = tempfile::tempdir().unwrap();
        let good = make_user_line("second line", "2026-04-22T10:00:00.000Z", None);
        write_jsonl(
            dir.path(),
            "malformed.jsonl",
            &["not json at all", &good],
        );

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "second line");
    }

    #[test]
    fn list_conversations_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("empty.jsonl"), "").unwrap();

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert!(
            result.is_empty(),
            "empty file must not yield a conversation"
        );
    }

    #[test]
    fn list_conversations_skips_slash_command_as_title() {
        let dir = tempfile::tempdir().unwrap();
        let slash = make_user_line(
            "<command-name>/clear</command-name><command-message>clearing</command-message>",
            "2026-04-22T10:00:00.000Z",
            None,
        );
        let real = make_user_line("fix the flaky test", "2026-04-22T10:01:00.000Z", None);
        write_jsonl(dir.path(), "slash.jsonl", &[&slash, &real]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "fix the flaky test");
    }

    #[test]
    fn list_conversations_handles_content_array() {
        let dir = tempfile::tempdir().unwrap();
        // JSONL requires each record on a single line — no embedded newlines.
        let content_array = r#"[{"type":"text","text":"part one"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"aGk="}},{"type":"text","text":"part two"}]"#;
        let raw = format!(
            "{{\"parentUuid\":null,\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":{content_array}}},\"timestamp\":\"2026-04-22T10:00:00.000Z\"}}"
        );
        write_jsonl(dir.path(), "array.jsonl", &[&raw]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "part one part two");
    }

    #[test]
    fn list_conversations_skips_attachment_first_line() {
        let dir = tempfile::tempdir().unwrap();
        // Line 0: a hook-style attachment record with no `message` field.
        let attach = r#"{"parentUuid":null,"type":"hook_success","hookName":"pre-prompt","data":{"msg":"ok"},"timestamp":"2026-04-22T09:59:00.000Z"}"#;
        // Line 1: another non-user record.
        let sys = r#"{"parentUuid":null,"type":"system","content":"boot","timestamp":"2026-04-22T09:59:30.000Z"}"#;
        // Line 2: junk (non-JSON).
        let junk = "garbage";
        // Line 3: the real user prompt.
        let real = make_user_line("the real prompt", "2026-04-22T10:00:00.000Z", None);
        write_jsonl(dir.path(), "hook.jsonl", &[attach, sys, junk, &real]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "the real prompt");
        // First-record meta came from the attachment (parentUuid null → root).
        assert!(result[0].parent_id.is_none());
    }

    #[test]
    fn list_conversations_skips_memory_subdir() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("memory")).unwrap();
        // Even put a file named memory.jsonl-like inside — must still be ignored.
        fs::write(
            dir.path().join("memory").join("summary.jsonl"),
            "ignored\n",
        )
        .unwrap();
        let line = make_user_line("hello", "2026-04-22T10:00:00.000Z", None);
        write_jsonl(dir.path(), "real.jsonl", &[&line]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "real");
    }

    #[test]
    fn list_conversations_skips_meta_user_messages() {
        let dir = tempfile::tempdir().unwrap();
        // isMeta: true → skip as title candidate even though type=user.
        let meta = r#"{"parentUuid":null,"type":"user","isMeta":true,"message":{"role":"user","content":"Caveat: session resumed"},"timestamp":"2026-04-22T09:59:00.000Z"}"#;
        let real = make_user_line("actual request", "2026-04-22T10:00:00.000Z", None);
        write_jsonl(dir.path(), "meta.jsonl", &[meta, &real]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "actual request");
    }

    #[test]
    fn list_conversations_falls_back_to_mtime_when_timestamp_missing() {
        let dir = tempfile::tempdir().unwrap();
        // No `timestamp` field on the first record.
        let raw = r#"{"parentUuid":null,"type":"user","message":{"role":"user","content":"no ts"}}"#;
        write_jsonl(dir.path(), "no-ts.jsonl", &[raw]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        // created_at should equal last_activity_at because we fell back.
        assert_eq!(result[0].created_at, result[0].last_activity_at);
    }

    #[test]
    fn list_conversations_missing_slug_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let result = list_conversations_in(&missing, Path::new("/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_conversations_ignores_non_jsonl() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("notes.txt"), "ignore me").unwrap();
        fs::write(dir.path().join("state.json"), "{}").unwrap();
        let line = make_user_line("real", "2026-04-22T10:00:00.000Z", None);
        write_jsonl(dir.path(), "good.jsonl", &[&line]);

        let result = list_conversations_in(dir.path(), Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "good");
    }

    // ─── unix mtime helper for tests ───

    /// Set `path`'s atime+mtime to `N` seconds before now.
    ///
    /// We don't pull in `filetime` just for two test cases — the workspace
    /// already has `libc` on unix, so `utimes(2)` is a one-liner.
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
        // Non-unix CI would need a different mechanism; tests that rely on
        // this helper currently run on macOS/Linux only.
    }
}
