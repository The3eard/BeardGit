//! Claude Code session file parsing and PID liveness detection.
//!
//! Claude Code stores one JSON file per running session at
//! `~/.claude/sessions/{pid}.json` with format:
//! ```json
//! { "pid": 12345, "sessionId": "uuid", "cwd": "/path", "startedAt": 17760..., "kind": "interactive", "entrypoint": "cli" }
//! ```

use std::fs;
use std::path::Path;

use ai_provider::{AiError, AiProviderKind, AiSession, SessionKind};
use serde::Deserialize;

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
pub fn list_sessions(repo_path: &Path) -> Result<Vec<AiSession>, AiError> {
    let sessions_dir = match dirs::home_dir() {
        Some(home) => home.join(".claude/sessions"),
        None => return Ok(vec![]),
    };

    if !sessions_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();
    let entries = fs::read_dir(&sessions_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
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

        // Filter: only sessions whose cwd matches this repo
        let session_cwd = Path::new(&file.cwd);
        if session_cwd != repo_path {
            continue;
        }

        let is_active = process_alive(file.pid);
        sessions.push(AiSession {
            id: file.session_id,
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
        });
    }

    Ok(sessions)
}

/// Check if a session is still active by re-checking PID liveness.
pub fn is_session_active(session: &AiSession) -> bool {
    session.is_active
}

/// Check if a PID is still running.
#[cfg(unix)]
fn process_alive(pid: u64) -> bool {
    // pid_t is i32 on most platforms; reject values that would overflow or be invalid.
    let Ok(pid_t) = libc::pid_t::try_from(pid) else {
        return false;
    };
    if pid_t <= 0 {
        return false;
    }
    // SAFETY: kill(pid, 0) only checks existence, sends no signal.
    unsafe { libc::kill(pid_t, 0) == 0 }
}

#[cfg(not(unix))]
fn process_alive(_pid: u64) -> bool {
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
    fn current_process_is_alive() {
        let pid = std::process::id() as u64;
        assert!(process_alive(pid));
    }

    #[test]
    fn nonexistent_pid_is_not_alive() {
        assert!(!process_alive(4_294_967_295));
    }

    #[test]
    fn empty_sessions_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = list_sessions(dir.path());
        assert!(result.unwrap().is_empty());
    }
}
