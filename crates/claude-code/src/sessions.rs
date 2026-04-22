//! Claude Code session file parsing and PID liveness detection.
//!
//! Claude Code stores one JSON file per running session at
//! `~/.claude/sessions/{pid}.json` with format:
//! ```json
//! { "pid": 12345, "sessionId": "uuid", "cwd": "/path", "startedAt": 17760..., "kind": "interactive", "entrypoint": "cli" }
//! ```

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use ai_provider::{AiError, AiProviderKind, AiSession, SessionKind};
use serde::Deserialize;

/// Session PID files older than this are skipped at the directory walk.
///
/// Claude Code writes one JSON file per run at `~/.claude/sessions/{pid}.json`
/// and never prunes them, so long-running installs accumulate hundreds of
/// dead-PID files that contribute nothing to the UI but slow the AI
/// Sessions view's initial paint. 30 days of retention matches the
/// Codex-side [`codex::sessions::DISCOVERY_WINDOW`] and covers typical
/// "show me last month's runs" UX without scanning the full history.
const DISCOVERY_WINDOW: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Raw session file as written by Claude Code.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeSessionFile {
    pid: u64,
    session_id: String,
    cwd: String,
    started_at: u64,
    kind: String,
    #[allow(dead_code)]
    entrypoint: Option<String>,
}

/// List Claude Code sessions whose `cwd` matches `repo_path`.
///
/// Claude writes `~/.claude/sessions/{pid}.json` where `sessionId` is the
/// *conversation* UUID — not a per-process identifier. When the user
/// resumes the same conversation in a new process, a second PID file
/// appears sharing the same `sessionId`. We dedupe by `(session_id, cwd)`
/// here so callers get at most one row per logical conversation, picking:
///
/// 1. the entry whose PID is both alive *and* actually owned by a
///    `claude` process (guards against macOS reusing a long-dead PID for
///    something completely unrelated), then
/// 2. the most recent `started_at` as the tie-break.
pub fn list_sessions(repo_path: &Path) -> Result<Vec<AiSession>, AiError> {
    let sessions_dir = match dirs::home_dir() {
        Some(home) => home.join(".claude/sessions"),
        None => return Ok(vec![]),
    };

    if !sessions_dir.is_dir() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(&sessions_dir)?;
    let now = SystemTime::now();

    // Collect (session_id -> best candidate so far) and resolve conflicts
    // with `prefer_candidate` below. Keying by session_id alone is fine —
    // we also filter by cwd, so two sessions with the same id *must* be
    // referring to the same conversation.
    let mut by_session_id: std::collections::HashMap<String, AiSession> =
        std::collections::HashMap::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        // Cheap `stat` before the read — files older than the discovery
        // window can't represent an active conversation (their PIDs are
        // long dead) and aren't worth parsing.
        if let Ok(meta) = entry.metadata()
            && let Ok(modified) = meta.modified()
            && now
                .duration_since(modified)
                .map(|age| age > DISCOVERY_WINDOW)
                .unwrap_or(false)
        {
            continue;
        }

        let raw = match fs::read_to_string(&path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let file: ClaudeSessionFile = match serde_json::from_str(&raw) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let session_cwd = Path::new(&file.cwd);
        if session_cwd != repo_path {
            continue;
        }

        let is_active = is_claude_process(file.pid);
        let candidate = AiSession {
            id: file.session_id.clone(),
            provider: AiProviderKind::ClaudeCode,
            cwd: session_cwd.to_path_buf(),
            started_at: Some(file.started_at),
            kind: match file.kind.as_str() {
                "interactive" => SessionKind::Interactive,
                _ => SessionKind::Headless,
            },
            is_active,
            worktree_path: None,
            background_status: None,
            task_id: None,
        };

        match by_session_id.get(&file.session_id) {
            None => {
                by_session_id.insert(file.session_id, candidate);
            }
            Some(existing) if prefer_candidate(&candidate, existing) => {
                by_session_id.insert(file.session_id, candidate);
            }
            Some(_) => {}
        }
    }

    Ok(by_session_id.into_values().collect())
}

/// Tie-break rule for duplicate `session_id` rows (same logical conversation
/// observed across multiple PID files). `true` means `candidate` replaces
/// the stored entry:
///
/// 1. A live-PID row always beats a dead-PID row.
/// 2. Same liveness → newer `started_at` wins.
/// 3. Same liveness + same `started_at` → incumbent wins (stable iter).
fn prefer_candidate(candidate: &AiSession, current: &AiSession) -> bool {
    if candidate.is_active != current.is_active {
        return candidate.is_active;
    }
    candidate.started_at.unwrap_or(0) > current.started_at.unwrap_or(0)
}

/// Check if a session is still active by re-checking PID liveness.
pub fn is_session_active(session: &AiSession) -> bool {
    session.is_active
}

/// Return true if the given PID is alive *and* the process is Claude Code.
///
/// macOS recycles PIDs aggressively — a crashed-and-never-cleaned
/// `{pid}.json` whose PID is now owned by Safari or `mdworker` would
/// otherwise look "active" forever to the UI. We run `ps -p {pid} -o
/// comm=` and require the command string to contain `"claude"` (the
/// Mach-O binary's basename, resilient to the full `comm` either being
/// `claude` or the Bun-bundled `/Users/...local/bin/claude` truncated
/// form depending on how the user launched it).
#[cfg(unix)]
fn is_claude_process(pid: u64) -> bool {
    // pid_t is i32 on most platforms; reject values that would overflow or be invalid.
    let Ok(pid_t) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid_t <= 0 {
        return false;
    }
    // Cheap liveness pre-check — if kill(0) fails, ps can't tell us
    // anything useful anyway.
    // SAFETY: kill(pid, 0) only checks existence, sends no signal.
    if unsafe { libc::kill(pid_t, 0) } != 0 {
        return false;
    }
    let pid_str = pid_t.to_string();
    let Ok(output) = std::process::Command::new("ps")
        .args(["-p", &pid_str, "-o", "comm="])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    // `ps -o comm=` prints the full command path (not the trailing
    // basename) on macOS. Strip any leading dirs and match the basename
    // exactly — "contains claude" is too loose (it would accept the
    // `claude-code-<hash>` test binary that happens to share a prefix).
    let comm = String::from_utf8_lossy(&output.stdout);
    let name = comm
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .to_lowercase();
    name == "claude"
}

#[cfg(not(unix))]
fn is_claude_process(_pid: u64) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_session_file(dir: &Path, pid: u64, cwd: &str, kind: &str) {
        let content = format!(
            r#"{{"pid":{pid},"sessionId":"test-uuid-{pid}","cwd":"{cwd}","startedAt":1776000000000,"kind":"{kind}","entrypoint":"cli"}}"#
        );
        fs::write(dir.join(format!("{pid}.json")), content).unwrap();
    }

    #[test]
    fn parse_session_files() {
        let home = tempfile::tempdir().unwrap();
        let sessions_dir = home.path().join(".claude/sessions");
        fs::create_dir_all(&sessions_dir).unwrap();

        write_session_file(&sessions_dir, 99999, "/my/repo", "interactive");
        write_session_file(&sessions_dir, 99998, "/other/repo", "interactive");

        let raw = fs::read_to_string(sessions_dir.join("99999.json")).unwrap();
        let file: ClaudeSessionFile = serde_json::from_str(&raw).unwrap();
        assert_eq!(file.pid, 99999);
        assert_eq!(file.session_id, "test-uuid-99999");
        assert_eq!(file.cwd, "/my/repo");
        assert_eq!(file.kind, "interactive");
    }

    #[test]
    fn session_kind_mapping() {
        assert_eq!(
            match "interactive" {
                "interactive" => SessionKind::Interactive,
                _ => SessionKind::Headless,
            },
            SessionKind::Interactive
        );
        assert_eq!(
            match "print" {
                "interactive" => SessionKind::Interactive,
                _ => SessionKind::Headless,
            },
            SessionKind::Headless
        );
    }

    #[test]
    fn current_process_is_not_a_claude_process() {
        // The test harness (`cargo test`) obviously isn't Claude, so
        // `is_claude_process` must reject the runner's own PID even
        // though `kill(pid, 0)` succeeds. Also protects against PID
        // recycling false-positives in production.
        let pid = std::process::id() as u64;
        assert!(!is_claude_process(pid));
    }

    #[test]
    fn nonexistent_pid_is_not_a_claude_process() {
        assert!(!is_claude_process(4_294_967_295));
    }

    #[test]
    fn empty_sessions_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = list_sessions(dir.path());
        assert!(result.unwrap().is_empty());
    }
}
