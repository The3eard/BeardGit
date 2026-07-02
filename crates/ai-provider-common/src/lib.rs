//! Shared helpers for BeardGit's CLI-based AI provider crates.
//!
//! Codex and OpenCode were structural twins: byte-for-byte-identical
//! directory-scan worktree discovery/cleanup, version-token parsing, and
//! commit-attribution heuristics that differed only in a handful of
//! provider-specific constants (binary name, worktree directory, attribution
//! needle). This crate hosts the single implementation, parameterized by
//! [`ProviderSpec`], so a worktree-cleanup or attribution fix lands once
//! instead of twice.
//!
//! Claude Code deliberately keeps its own `worktrees` / `attribution` modules:
//! it discovers worktrees via `git worktree list --porcelain` (a `worktree-`
//! branch prefix, not a directory scan) and matches a richer attribution
//! pattern set (`Authored-by:` footer + Claude/Anthropic), neither of which
//! fits this shape — forcing them together would be a bad unification. It does
//! share the one genuinely-identical piece, [`parse_version_token`].
//!
//! This crate uses only `std::process::Command` + the filesystem and lives
//! outside the trait-purity-guarded crate paths (`provider` / `forge-provider`
//! / `ai-provider`), so it can carry this runtime logic freely.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ai_provider::{AiError, AiProviderKind, AiWorktree, WorktreeStatus};

/// Relative path of the optional session-id marker inside each worktree.
const SESSION_MARKER_REL: &str = ".beardgit/ai-session";

/// Provider-specific constants that parameterize the shared helpers.
///
/// One `const ProviderSpec` lives in each provider crate; the helpers below
/// take it by reference so the behaviour stays identical across providers
/// except for these values.
pub struct ProviderSpec {
    /// Which provider these helpers act on (stamped onto every [`AiWorktree`]).
    pub kind: AiProviderKind,
    /// Binary name resolved on `PATH` by [`detect_binary`].
    pub binary_name: &'static str,
    /// Repo-root artifact directory that marks the provider as "used here"
    /// (e.g. `.codex` / `.opencode`), checked by [`detect_in_repo`].
    pub repo_marker_dir: &'static str,
    /// Relative path (from the repo root) under which BeardGit spawns this
    /// provider's worktrees (e.g. `.beardgit/ai-worktrees/codex`).
    pub worktree_dir: &'static str,
    /// Lowercase needle matched against commit trailers / author name for
    /// attribution (e.g. `codex` / `opencode`).
    pub attribution_needle: &'static str,
}

// ─── Detection ──────────────────────────────────────────────────────────────

/// Find the provider's binary on `PATH`.
pub fn detect_binary(spec: &ProviderSpec) -> Option<PathBuf> {
    which::which(spec.binary_name).ok()
}

/// Check whether the provider has an artifact directory in the given repo root.
pub fn detect_in_repo(spec: &ProviderSpec, repo_path: &Path) -> bool {
    repo_path.join(spec.repo_marker_dir).is_dir()
}

/// Extract a semver-like version token from `<binary> --version` output.
///
/// Scans for the first whitespace-delimited token that starts with an ASCII
/// digit and contains a `.`. Returns `None` when no token matches. This is the
/// one detection helper genuinely shared with Claude Code too.
pub fn parse_version_token(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        for token in trimmed.split_whitespace() {
            if token.chars().next().is_some_and(|c| c.is_ascii_digit()) && token.contains('.') {
                return Some(token.to_string());
            }
        }
    }
    None
}

/// Run `<binary> --version` and return the parsed version token.
///
/// Errors with [`AiError::CommandBuild`] if the process fails to spawn and
/// [`AiError::Parse`] if no version token is present in the output.
pub fn version(binary: &Path) -> Result<String, AiError> {
    let output = Command::new(binary)
        .arg("--version")
        .output()
        .map_err(|e| AiError::CommandBuild(format!("failed to run --version: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version_token(&stdout)
        .ok_or_else(|| AiError::Parse(format!("no version found in output: {stdout}")))
}

// ─── Directory-scan worktrees (Codex / OpenCode) ─────────────────────────────

/// List all provider worktrees spawned by BeardGit under `spec.worktree_dir`.
///
/// Returns `Ok(Vec::new())` when the parent directory doesn't exist — a
/// brand-new repo with zero BeardGit activity is the common case. Each
/// subdirectory corresponds to one worktree; the optional
/// `.beardgit/ai-session` marker file holds the session id.
pub fn list_worktrees(spec: &ProviderSpec, repo_path: &Path) -> Result<Vec<AiWorktree>, AiError> {
    let root = repo_path.join(spec.worktree_dir);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut worktrees = Vec::new();
    let entries = fs::read_dir(&root).map_err(AiError::Io)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let branch = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let session_id = read_session_marker(&path);
        let status = determine_status(&path);
        worktrees.push(AiWorktree {
            path,
            branch,
            provider: spec.kind,
            session_id,
            status,
        });
    }
    Ok(worktrees)
}

/// Remove the given worktree directory recursively.
///
/// These worktrees are plain directories (not necessarily linked git
/// worktrees), so a simple `remove_dir_all` is sufficient. Cleanup is
/// idempotent: a non-existent path is a no-op.
pub fn cleanup_worktree(worktree: &AiWorktree) -> Result<(), AiError> {
    if worktree.path.exists() {
        fs::remove_dir_all(&worktree.path).map_err(AiError::Io)?;
    }
    Ok(())
}

/// Read the optional `.beardgit/ai-session` marker file if present.
fn read_session_marker(worktree_path: &Path) -> Option<String> {
    let marker = worktree_path.join(SESSION_MARKER_REL);
    fs::read_to_string(marker).ok().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Classify a worktree as Active / Clean / Orphaned.
///
/// - **Active**: session marker present (liveness is not probed here).
/// - **Clean**: directory exists but no session marker.
/// - **Orphaned**: path doesn't exist.
fn determine_status(path: &Path) -> WorktreeStatus {
    if !path.is_dir() {
        return WorktreeStatus::Orphaned;
    }
    let marker = path.join(SESSION_MARKER_REL);
    if marker.is_file() {
        WorktreeStatus::Active
    } else {
        WorktreeStatus::Clean
    }
}

// ─── Attribution (simple needle) ─────────────────────────────────────────────

/// Return `true` when `message` / `author` look like they came from the
/// provider identified by `spec.attribution_needle`.
///
/// Matches a `Co-authored-by:` trailer containing the needle, or an author
/// name containing it. Case-insensitive and conservative — under-reporting is
/// preferred over mislabelling a human-authored commit.
pub fn is_ai_authored(spec: &ProviderSpec, message: &str, author: &str) -> bool {
    let needle = spec.attribution_needle;
    for line in message.lines() {
        let lower = line.trim().to_lowercase();
        if lower.starts_with("co-authored-by:") && lower.contains(needle) {
            return true;
        }
    }
    author.to_lowercase().contains(needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SPEC: ProviderSpec = ProviderSpec {
        kind: AiProviderKind::Codex,
        binary_name: "codex",
        repo_marker_dir: ".codex",
        worktree_dir: ".beardgit/ai-worktrees/codex",
        attribution_needle: "codex",
    };

    fn mkwt(repo: &Path, slug: &str) -> PathBuf {
        let path = repo.join(TEST_SPEC.worktree_dir).join(slug);
        fs::create_dir_all(&path).unwrap();
        path
    }

    // ── detection ──

    #[test]
    fn detect_in_repo_true_when_marker_dir_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".codex")).unwrap();
        assert!(detect_in_repo(&TEST_SPEC, dir.path()));
    }

    #[test]
    fn detect_in_repo_false_for_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!detect_in_repo(&TEST_SPEC, dir.path()));
    }

    #[test]
    fn parse_version_standard_format() {
        assert_eq!(parse_version_token("codex 0.1.2").unwrap(), "0.1.2");
    }

    #[test]
    fn parse_version_version_only() {
        assert_eq!(parse_version_token("0.1.2\n").unwrap(), "0.1.2");
    }

    #[test]
    fn parse_version_multiline() {
        assert_eq!(
            parse_version_token("Codex CLI\nVersion: 0.1.2\n").unwrap(),
            "0.1.2"
        );
    }

    #[test]
    fn parse_version_no_version() {
        assert!(parse_version_token("no version here").is_none());
    }

    // ── worktrees ──

    #[test]
    fn empty_repo_returns_no_worktrees() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_worktrees(&TEST_SPEC, dir.path()).unwrap().is_empty());
    }

    #[test]
    fn lists_every_subdir_as_worktree() {
        let dir = tempfile::tempdir().unwrap();
        mkwt(dir.path(), "feat-a");
        mkwt(dir.path(), "feat-b");
        let wts = list_worktrees(&TEST_SPEC, dir.path()).unwrap();
        assert_eq!(wts.len(), 2);
        let branches: Vec<_> = wts.iter().map(|w| w.branch.clone()).collect();
        assert!(branches.contains(&"feat-a".to_string()));
        assert!(branches.contains(&"feat-b".to_string()));
        assert!(wts.iter().all(|w| w.provider == AiProviderKind::Codex));
    }

    #[test]
    fn reads_optional_session_marker() {
        let dir = tempfile::tempdir().unwrap();
        let wt = mkwt(dir.path(), "feat-a");
        let marker_dir = wt.join(".beardgit");
        fs::create_dir_all(&marker_dir).unwrap();
        fs::write(marker_dir.join("ai-session"), "sess-123\n").unwrap();

        let wts = list_worktrees(&TEST_SPEC, dir.path()).unwrap();
        assert_eq!(wts.len(), 1);
        assert_eq!(wts[0].session_id.as_deref(), Some("sess-123"));
        assert_eq!(wts[0].status, WorktreeStatus::Active);
    }

    #[test]
    fn worktree_without_marker_is_clean() {
        let dir = tempfile::tempdir().unwrap();
        mkwt(dir.path(), "feat-a");
        let wts = list_worktrees(&TEST_SPEC, dir.path()).unwrap();
        assert_eq!(wts.len(), 1);
        assert!(wts[0].session_id.is_none());
        assert_eq!(wts[0].status, WorktreeStatus::Clean);
    }

    #[test]
    fn ignores_files_in_worktree_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(TEST_SPEC.worktree_dir)).unwrap();
        fs::write(
            dir.path().join(TEST_SPEC.worktree_dir).join("notes.txt"),
            "hi",
        )
        .unwrap();
        assert!(list_worktrees(&TEST_SPEC, dir.path()).unwrap().is_empty());
    }

    #[test]
    fn cleanup_worktree_removes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let wt = mkwt(dir.path(), "feat-clean");
        fs::write(wt.join("file.txt"), "data").unwrap();
        let worktree = AiWorktree {
            path: wt.clone(),
            branch: "feat-clean".into(),
            provider: AiProviderKind::Codex,
            session_id: None,
            status: WorktreeStatus::Clean,
        };
        cleanup_worktree(&worktree).unwrap();
        assert!(!wt.exists());
    }

    #[test]
    fn cleanup_worktree_missing_path_is_ok() {
        let worktree = AiWorktree {
            path: PathBuf::from("/nonexistent/codex/worktree"),
            branch: "ghost".into(),
            provider: AiProviderKind::Codex,
            session_id: None,
            status: WorktreeStatus::Orphaned,
        };
        cleanup_worktree(&worktree).unwrap();
    }

    // ── attribution ──

    #[test]
    fn trailer_with_needle_matches() {
        let msg = "feat: thing\n\nCo-authored-by: Codex CLI <codex@openai.com>\n";
        assert!(is_ai_authored(&TEST_SPEC, msg, "Alice"));
    }

    #[test]
    fn author_containing_needle_matches() {
        assert!(is_ai_authored(
            &TEST_SPEC,
            "fix: x",
            "OpenAI Codex <codex@openai.com>"
        ));
    }

    #[test]
    fn human_commit_not_matched() {
        assert!(!is_ai_authored(
            &TEST_SPEC,
            "feat: add feature\n\nSigned-off-by: Alice",
            "Alice"
        ));
    }

    #[test]
    fn case_insensitive_trailer() {
        let msg = "fix: bug\n\nCO-AUTHORED-BY: CODEX <codex@openai.com>";
        assert!(is_ai_authored(&TEST_SPEC, msg, "Alice"));
    }
}
