//! OpenCode session discovery via the `opencode session list --format json`
//! CLI command.
//!
//! Unlike Codex (where we parse JSONL rollout files directly from
//! `~/.codex/sessions/`), OpenCode stores its sessions in a SQLite DB at
//! `~/.local/share/opencode/opencode.db`. Rather than depending on SQLite
//! and risking schema drift, we shell out to the first-class `opencode
//! session list --format json` command and parse the JSON array.
//!
//! **Probe verified on 2026-04-20** against `opencode 1.4.10`. A real
//! invocation in `/tmp/opencode-test` produced:
//!
//! ```json
//! [
//!   {
//!     "id": "ses_25317d847ffePGEr3P1tJRkVYa",
//!     "title": "New session - 2026-04-20T21:59:48.920Z",
//!     "updated": 1776722389315,
//!     "created": 1776722388920,
//!     "projectId": "global",
//!     "directory": "/private/tmp/opencode-test"
//!   }
//! ]
//! ```
//!
//! `updated` and `created` are Unix epoch **milliseconds**. `directory` is the
//! session's working directory when created; OpenCode's per-session `cwd`
//! isn't a first-class filter arg for `session list`, so this module returns
//! ALL sessions — the provider layer can filter later if needed.
//!
//! All CLI shell-outs go through the [`SessionRunner`] trait so unit tests
//! can replay canned JSON without actually spawning `opencode`.
//!
//! Session **liveness** is inferred from each record's `updated` epoch-ms
//! value: if the session was touched in the last [`ACTIVE_WINDOW`] we
//! report it as active. OpenCode does not record a PID in its session
//! metadata, so this recency heuristic — the same one used by the Codex
//! integration — is the closest proxy we have.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ai_provider::{AiProviderKind, AiSession, SessionKind};
use serde::Deserialize;

/// How stale an OpenCode session's `updated` timestamp may be before it's
/// considered inactive. Matches the 5-minute window used in the Codex
/// provider so both integrations share a uniform liveness policy.
pub const ACTIVE_WINDOW: Duration = Duration::from_secs(5 * 60);

/// Abstract runner for the `opencode` CLI.
///
/// Implemented by [`CliSessionRunner`] in production and by test doubles in
/// unit tests. The trait only needs to shell out to `opencode <args>` and
/// return combined stdout.
pub trait SessionRunner {
    /// Run `opencode` with the given args; return stdout as a `String`.
    fn run(&self, args: &[&str]) -> std::io::Result<String>;
}

/// Production runner that spawns `opencode` on PATH with `--log-level ERROR`
/// to silence the default INFO chatter that otherwise ends up on stderr.
pub struct CliSessionRunner {
    binary: PathBuf,
}

impl CliSessionRunner {
    /// Create a runner bound to the given `opencode` binary path.
    pub fn new(binary: PathBuf) -> Self {
        Self { binary }
    }
}

impl SessionRunner for CliSessionRunner {
    fn run(&self, args: &[&str]) -> std::io::Result<String> {
        let output = Command::new(&self.binary)
            .args(args)
            .arg("--log-level")
            .arg("ERROR")
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// JSON shape returned by `opencode session list --format json`.
#[derive(Debug, Deserialize)]
struct RawSession {
    id: String,
    #[serde(default)]
    #[allow(dead_code)]
    title: Option<String>,
    /// Unix epoch **milliseconds** when the session was created.
    #[serde(default)]
    created: Option<u64>,
    /// Unix epoch **milliseconds** of the last update. Drives the
    /// [`AiSession::is_active`] liveness flag via [`is_updated_active`].
    #[serde(default)]
    updated: Option<u64>,
    /// Working directory at session creation. May be absent on older
    /// OpenCode versions — we fall back to "" (empty path) in that case.
    #[serde(default)]
    directory: Option<String>,
}

/// Shell out to `opencode session list --format json` via `runner` and
/// return every session parsed from the JSON array.
///
/// Returns an empty `Vec` if the CLI exits non-zero, stdout is empty, or
/// the JSON is malformed — the provider surface is "best-effort": we never
/// crash the UI just because the sidecar's output changed shape.
///
/// Because OpenCode's `session list` doesn't take a `cwd` filter, this
/// returns ALL sessions regardless of repo. The provider layer may filter
/// later on `AiSession.cwd` if per-repo scoping is required.
///
/// TODO: once OpenCode adds a `--cwd` / `--project` filter we can narrow
/// at the CLI boundary and drop the full scan.
pub fn load_sessions(runner: &dyn SessionRunner) -> Vec<AiSession> {
    let stdout = match runner.run(&["session", "list", "--format", "json"]) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    parse_sessions(&stdout)
}

/// Parse the raw JSON array into [`AiSession`] values.
///
/// Extracted so unit tests can feed fixtures without the runner indirection.
fn parse_sessions(stdout: &str) -> Vec<AiSession> {
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let raw: Vec<RawSession> = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    raw.into_iter().map(to_ai_session).collect()
}

/// Convert a [`RawSession`] into an [`AiSession`].
///
/// `is_active` is derived from the `updated` timestamp via
/// [`is_updated_active`]: sessions touched within [`ACTIVE_WINDOW`] of
/// now count as live.
fn to_ai_session(r: RawSession) -> AiSession {
    let cwd = r
        .directory
        .as_deref()
        .map(Path::new)
        .unwrap_or(Path::new(""))
        .to_path_buf();

    let is_active = r.updated.map(is_updated_active).unwrap_or(false);

    AiSession {
        id: r.id,
        provider: AiProviderKind::OpenCode,
        cwd,
        // OpenCode's `created` is already Unix millis, which matches
        // `AiSession::started_at`'s contract.
        started_at: r.created,
        // `opencode session list` doesn't distinguish interactive vs. run:
        // treat every listed session as Interactive by default — background
        // runs spawned by BeardGit set this field explicitly elsewhere.
        kind: SessionKind::Interactive,
        is_active,
        worktree_path: None,
        background_status: None,
        task_id: None,
    }
}

/// True when a session's `updated` millis value is within [`ACTIVE_WINDOW`]
/// of now. Also returns `true` if the timestamp is in the future (clock
/// skew — better to show a session as active than to drop it).
pub fn is_updated_active(updated_millis: u64) -> bool {
    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    if updated_millis > now_millis {
        return true;
    }
    let age = Duration::from_millis(now_millis - updated_millis);
    age <= ACTIVE_WINDOW
}

/// Convenience entry point used by [`crate::OpenCodeProvider::is_session_active`].
///
/// Re-shells-out through `runner` and scans the current `session list`
/// for a session whose id matches and whose `updated` is recent. Returns
/// `false` when the runner fails or no match is found.
pub fn is_session_active_by_id(runner: &dyn SessionRunner, session_id: &str) -> bool {
    let sessions = load_sessions(runner);
    sessions.iter().any(|s| s.id == session_id && s.is_active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Deterministic mock runner that returns pre-canned stdout for tests.
    pub(super) struct MockSessionRunner {
        stdout: String,
        calls: RefCell<Vec<Vec<String>>>,
    }

    impl MockSessionRunner {
        pub(super) fn new(stdout: impl Into<String>) -> Self {
            Self {
                stdout: stdout.into(),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl SessionRunner for MockSessionRunner {
        fn run(&self, args: &[&str]) -> std::io::Result<String> {
            self.calls
                .borrow_mut()
                .push(args.iter().map(|s| s.to_string()).collect());
            Ok(self.stdout.clone())
        }
    }

    #[test]
    fn parses_single_session() {
        let fx = r#"[{"id":"ses_25317d847","title":"New session","updated":1776722389315,"created":1776722388920,"projectId":"global","directory":"/tmp/oc"}]"#;
        let runner = MockSessionRunner::new(fx);
        let sessions = load_sessions(&runner);
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.id, "ses_25317d847");
        assert_eq!(s.cwd, Path::new("/tmp/oc"));
        assert_eq!(s.provider, AiProviderKind::OpenCode);
        assert_eq!(s.started_at, Some(1_776_722_388_920));
        assert_eq!(s.kind, SessionKind::Interactive);
    }

    #[test]
    fn passes_expected_args_to_runner() {
        let runner = MockSessionRunner::new("[]");
        let _ = load_sessions(&runner);
        let calls = runner.calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            vec!["session", "list", "--format", "json"],
            "load_sessions must pass `session list --format json`"
        );
    }

    #[test]
    fn empty_array_returns_empty_vec() {
        let runner = MockSessionRunner::new("[]");
        assert!(load_sessions(&runner).is_empty());
    }

    #[test]
    fn empty_stdout_returns_empty_vec() {
        let runner = MockSessionRunner::new("");
        assert!(load_sessions(&runner).is_empty());
    }

    #[test]
    fn malformed_json_returns_empty_vec_without_panic() {
        let runner = MockSessionRunner::new("not-json{{");
        assert!(load_sessions(&runner).is_empty());
    }

    #[test]
    fn missing_directory_becomes_empty_path() {
        let fx = r#"[{"id":"ses_x","updated":1,"created":1}]"#;
        let runner = MockSessionRunner::new(fx);
        let sessions = load_sessions(&runner);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].cwd, Path::new(""));
    }

    /// Build a fixture JSON string with one session whose `updated` is `millis`.
    fn fixture_with_updated(millis: u64) -> String {
        format!(
            r#"[{{"id":"ses_abc","title":"t","updated":{millis},"created":{millis},"projectId":"global","directory":"/tmp/foo"}}]"#
        )
    }

    #[test]
    fn recent_updated_is_active() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let runner = MockSessionRunner::new(fixture_with_updated(now));
        let sessions = load_sessions(&runner);
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].is_active, "fresh session must be active");
    }

    #[test]
    fn old_updated_is_inactive() {
        // One hour ago, well beyond the 5-minute window.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let old = now.saturating_sub(3_600_000);
        let runner = MockSessionRunner::new(fixture_with_updated(old));
        let sessions = load_sessions(&runner);
        assert_eq!(sessions.len(), 1);
        assert!(!sessions[0].is_active, "stale session must not be active");
    }

    #[test]
    fn is_updated_active_future_timestamp_is_active() {
        // Clock skew: future updated should not crash and should be treated
        // as active.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        assert!(is_updated_active(now + 60_000));
    }

    #[test]
    fn is_updated_active_boundary_at_window() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        // Just inside the window → active.
        assert!(is_updated_active(now.saturating_sub(4 * 60_000)));
        // Just outside → inactive.
        assert!(!is_updated_active(now.saturating_sub(10 * 60_000)));
    }

    #[test]
    fn is_session_active_by_id_finds_matching_fresh_session() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let runner = MockSessionRunner::new(fixture_with_updated(now));
        assert!(is_session_active_by_id(&runner, "ses_abc"));
        assert!(!is_session_active_by_id(&runner, "no-such-id"));
    }

    #[test]
    fn is_session_active_by_id_skips_stale_match() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let old = now.saturating_sub(3_600_000);
        let runner = MockSessionRunner::new(fixture_with_updated(old));
        // id matches but `updated` is stale → not active.
        assert!(!is_session_active_by_id(&runner, "ses_abc"));
    }

    #[test]
    fn multiple_sessions_are_all_parsed() {
        let fx = r#"[
            {"id":"a","updated":1,"created":1,"directory":"/tmp/a"},
            {"id":"b","updated":2,"created":2,"directory":"/tmp/b"}
        ]"#;
        let runner = MockSessionRunner::new(fx);
        let sessions = load_sessions(&runner);
        assert_eq!(sessions.len(), 2);
        let ids: Vec<_> = sessions.iter().map(|s| s.id.clone()).collect();
        assert!(ids.contains(&"a".to_string()));
        assert!(ids.contains(&"b".to_string()));
    }
}
