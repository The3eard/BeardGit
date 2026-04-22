//! OpenCode conversation transcript discovery.
//!
//! Shells out to `opencode session list --format json` — the same CLI that
//! [`super::sessions`] uses — and adapts the result into the transcript-first
//! [`AiConversation`] shape expected by the AI Sessions UI. This is the
//! moral equivalent of [`super::sessions::load_sessions`] but:
//!
//! * returns [`AiConversation`] rather than [`ai_provider::AiSession`],
//! * filters by `repo_path` (matching `directory` from the JSON with
//!   trailing-slash normalisation),
//! * enforces the same 30-day [`DISCOVERY_WINDOW`] used by the Claude
//!   Code and Codex listers so all providers share a uniform surface,
//! * sorts descending by `updated` (unix-ms) for the UI sidebar,
//! * sets `parent_id` to `None` — OpenCode does not fork conversations.
//!
//! The legacy PID-scan path in [`super::sessions`] stays in place and is
//! removed in Phase 8; both listers share the [`SessionRunner`] trait so
//! tests can replay canned JSON without spawning `opencode`.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ai_provider::{AiConversation, AiError, AiProviderKind};
use serde::Deserialize;

use crate::sessions::SessionRunner;

/// Conversations whose `updated` mtime is older than this are skipped.
///
/// Matches the 30-day window used by [`claude_code::conversations`] and
/// [`super::conversations_codex`-style] Codex listers so the UI shows a
/// consistent "recent activity" horizon across providers. OpenCode's
/// `session list` does not prune on its own, so without this the sidebar
/// would surface multi-year-old sessions on every refresh.
pub const DISCOVERY_WINDOW: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// JSON shape returned by `opencode session list --format json`.
///
/// Field names verified on 2026-04-20 against `opencode 1.4.10`.
/// Unknown keys (e.g. `projectId`) are ignored — we only read the fields
/// we map to [`AiConversation`].
#[derive(Debug, Deserialize)]
struct RawSession {
    /// Provider-native session UUID (e.g. `ses_25317d847…`).
    id: String,
    /// Title OpenCode computed for the session. May be `null` / absent
    /// on very new sessions — mapped to the empty string in that case.
    #[serde(default)]
    title: Option<String>,
    /// Unix epoch **milliseconds** when the session was created.
    #[serde(default)]
    created: Option<u64>,
    /// Unix epoch **milliseconds** of the most recent activity.
    #[serde(default)]
    updated: Option<u64>,
    /// Working directory at session creation.
    #[serde(default)]
    directory: Option<String>,
}

/// List OpenCode conversations scoped to `repo_path`.
///
/// Shells out via `runner`, parses the JSON array, drops entries whose
/// `directory` doesn't match `repo_path` (trailing-slash-normalised),
/// discards entries older than [`DISCOVERY_WINDOW`], and returns the
/// remainder sorted descending by `updated`.
///
/// Returns `Ok(vec![])` on any runner error (e.g. binary not installed,
/// non-zero exit) or unparseable JSON — matches the tolerance used by
/// [`super::sessions::load_sessions`]: we never crash the UI because the
/// sidecar's output shape changed.
pub fn list_conversations(
    runner: &dyn SessionRunner,
    repo_path: &Path,
) -> Result<Vec<AiConversation>, AiError> {
    let stdout = match runner.run(&["session", "list", "--format", "json"]) {
        Ok(s) => s,
        Err(_) => return Ok(vec![]),
    };
    Ok(parse_conversations(&stdout, repo_path))
}

/// Parse raw JSON into filtered + sorted [`AiConversation`]s.
///
/// Extracted from [`list_conversations`] so tests can feed fixtures
/// directly without the runner indirection.
fn parse_conversations(stdout: &str, repo_path: &Path) -> Vec<AiConversation> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let raw: Vec<RawSession> = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let window_millis = DISCOVERY_WINDOW.as_millis() as u64;

    let mut out: Vec<AiConversation> = raw
        .into_iter()
        .filter_map(|r| to_conversation(r, repo_path, now_millis, window_millis))
        .collect();

    // Newest activity first — matches the Claude Code / Codex listers
    // and the sidebar's UX expectation.
    out.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
    out
}

/// Convert a [`RawSession`] to [`AiConversation`] when it passes the
/// `repo_path` and discovery-window filters; otherwise `None`.
fn to_conversation(
    r: RawSession,
    repo_path: &Path,
    now_millis: u64,
    window_millis: u64,
) -> Option<AiConversation> {
    let directory = r.directory.as_deref()?;
    let cwd = PathBuf::from(directory);
    if !cwd_matches(&cwd, repo_path) {
        return None;
    }

    let updated = r.updated?;
    // Only enforce the window when `updated` is in the past; future
    // timestamps (clock skew) fall through as "recent".
    if updated < now_millis && now_millis - updated > window_millis {
        return None;
    }

    let created_at = r.created.unwrap_or(updated);
    let title = r.title.unwrap_or_default();

    Some(AiConversation {
        id: r.id,
        provider: AiProviderKind::OpenCode,
        cwd,
        created_at,
        last_activity_at: updated,
        title,
        // OpenCode does not fork conversations — no parent pointer.
        parent_id: None,
    })
}

/// True when `candidate` equals `repo` or sits beneath it.
///
/// Both sides get a single trailing `/` stripped (except bare root) before
/// comparison, so a session recorded at `/repo/` still matches a
/// `repo_path` of `/repo`.
fn cwd_matches(candidate: &Path, repo: &Path) -> bool {
    let c = normalise(candidate);
    let r = normalise(repo);
    c == r || c.starts_with(&r)
}

/// Strip a single trailing `/` off a path string, preserving bare root.
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
    use std::cell::RefCell;

    /// Deterministic mock runner that returns pre-canned stdout for tests.
    ///
    /// Mirrors the shape used in [`super::sessions`]'s tests so both
    /// listers share a single test harness pattern.
    struct MockSessionRunner {
        stdout: std::io::Result<String>,
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl MockSessionRunner {
        fn ok(stdout: impl Into<String>) -> Self {
            Self {
                stdout: Ok(stdout.into()),
                calls: RefCell::new(Vec::new()),
            }
        }

        fn err() -> Self {
            Self {
                stdout: Err(std::io::Error::other("boom")),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl SessionRunner for MockSessionRunner {
        fn run(&self, args: &[&str]) -> std::io::Result<String> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|s| s.to_string()).collect());
            match &self.stdout {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(std::io::Error::new(e.kind(), e.to_string())),
            }
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[test]
    fn parses_opencode_session_list_json() {
        let now = now_ms();
        let fx = format!(
            r#"[{{"id":"ses_1","title":"fix flaky test","updated":{now},"created":{created},"projectId":"global","directory":"/my/repo"}}]"#,
            created = now - 1000,
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
        let c = &result[0];
        assert_eq!(c.id, "ses_1");
        assert_eq!(c.provider, AiProviderKind::OpenCode);
        assert_eq!(c.cwd, Path::new("/my/repo"));
        assert_eq!(c.title, "fix flaky test");
        assert_eq!(c.last_activity_at, now);
        assert_eq!(c.created_at, now - 1000);
        assert!(c.parent_id.is_none());
        // Verify the CLI shape we actually invoked.
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], vec!["session", "list", "--format", "json"]);
    }

    #[test]
    fn filters_by_repo_path() {
        let now = now_ms();
        let fx = format!(
            r#"[
                {{"id":"a","updated":{now},"created":{now},"directory":"/my/repo"}},
                {{"id":"b","updated":{now},"created":{now},"directory":"/some/other"}}
            ]"#
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "a");
    }

    #[test]
    fn matches_subdirectory_cwd() {
        let now = now_ms();
        let fx = format!(
            r#"[{{"id":"wt","updated":{now},"created":{now},"directory":"/repo/worktree-1"}}]"#
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].cwd, Path::new("/repo/worktree-1"));
    }

    #[test]
    fn trailing_slash_normalised_on_comparison() {
        let now = now_ms();
        let fx = format!(
            r#"[{{"id":"trail","updated":{now},"created":{now},"directory":"/my/repo/"}}]"#
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn handles_missing_title() {
        let now = now_ms();
        // Two rows: one with `title` absent entirely, one with `title: null`.
        let fx = format!(
            r#"[
                {{"id":"a","updated":{now},"created":{now},"directory":"/my/repo"}},
                {{"id":"b","title":null,"updated":{now},"created":{now},"directory":"/my/repo"}}
            ]"#
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 2);
        for c in &result {
            assert_eq!(c.title, "");
        }
    }

    #[test]
    fn sorts_desc_by_updated() {
        let now = now_ms();
        let older = now - 10_000;
        let newer = now - 1_000;
        let fx = format!(
            r#"[
                {{"id":"older","updated":{older},"created":{older},"directory":"/my/repo"}},
                {{"id":"newer","updated":{newer},"created":{newer},"directory":"/my/repo"}}
            ]"#
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "newer");
        assert_eq!(result[1].id, "older");
        assert!(result[0].last_activity_at >= result[1].last_activity_at);
    }

    #[test]
    fn skips_old_sessions() {
        let now = now_ms();
        // 40 days ago — well outside the 30-day window.
        let stale = now - 40 * 24 * 60 * 60 * 1_000;
        let fresh = now - 1_000;
        let fx = format!(
            r#"[
                {{"id":"stale","updated":{stale},"created":{stale},"directory":"/my/repo"}},
                {{"id":"fresh","updated":{fresh},"created":{fresh},"directory":"/my/repo"}}
            ]"#
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "fresh");
    }

    #[test]
    fn empty_when_runner_errors() {
        let runner = MockSessionRunner::err();
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn empty_when_json_malformed() {
        let runner = MockSessionRunner::ok("not-json{{");
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn empty_when_stdout_empty() {
        let runner = MockSessionRunner::ok("");
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn skips_entries_missing_directory() {
        let now = now_ms();
        // Missing `directory` → filtered out even if the rest is valid.
        let fx = format!(r#"[{{"id":"nodir","updated":{now},"created":{now}}}]"#);
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn future_updated_survives_window_filter() {
        // Clock skew guard: `updated` in the future should still be
        // surfaced rather than dropped by the age check.
        let now = now_ms();
        let future = now + 60_000;
        let fx = format!(
            r#"[{{"id":"future","updated":{future},"created":{future},"directory":"/my/repo"}}]"#
        );
        let runner = MockSessionRunner::ok(fx);
        let result = list_conversations(&runner, Path::new("/my/repo")).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "future");
    }
}
