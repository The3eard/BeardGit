//! `clone_repo` — validates a clone request and hands the actual clone to
//! the [`TaskManager`], returning where the repo will land plus the id of
//! the task doing the work.
//!
//! The clone shells out to `git clone <url> <target>` so cred-helpers
//! (`gh auth`, `glab auth`, `osxkeychain`, …) and SSH agents Just Work the
//! same way they do everywhere else in BeardGit. We intentionally do not
//! use `git2`'s built-in clone here: libgit2 cannot reuse the user's
//! configured credential helpers, which would give us a worse UX than the
//! status quo (where the user runs `git clone` in a terminal).
//!
//! Validation stays in this command and stays synchronous — it is pure
//! string and `stat` work, and the dialog wants those failures back in its
//! banner before it closes. The clone itself does not: it used to run
//! inline in a non-async command, which Tauri executes on the main thread,
//! so the window was frozen for however long the clone took, with no
//! progress and no way to cancel. It now goes through `TaskManager` like
//! fetch / pull / push, which is also what finally gives
//! [`task_runner::TaskKind::GitClone`] a producer — the whole drawer path
//! for it (allowlist, wire kind, row, detail panel, i18n) already existed
//! and nothing ever created the task.
//!
//! The validation errors stay tagged so the dialog banner can branch on the
//! failure mode without parsing free text — same convention as
//! [`super::init::InitRepoError`]. A failure of the clone itself is no
//! longer one of them: it arrives as a failed task.
//!
//! The task is spawned `cancellable`, which is new — an unwanted clone of a
//! huge repo used to be unstoppable. One consequence: `git clone` cleans up
//! its target after failing on its own, but not after being killed, so a
//! cancelled clone leaves a partial checkout behind. Retrying then trips
//! `DestinationExists`, whose message already tells the user to remove it or
//! pick another folder. We deliberately do not delete it for them — silently
//! removing a directory the user did not name is its own class of bug.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use task_runner::{SpawnOptions, TaskId, TaskKind, TaskManager};
use tauri::State;

use crate::ipc_error::IpcError;

/// Options accepted by [`clone_repo`] (and [`validate_clone_request`]).
#[derive(Debug, Deserialize)]
pub struct CloneRepoOptions {
    /// Clone URL — HTTPS, SSH, or `git@` shorthand.
    pub url: String,
    /// Absolute path to the *parent* folder where the repo should land.
    /// The final folder name is derived from `url` and created as a
    /// subdirectory of `parent_dir`.
    pub parent_dir: String,
}

/// Accepted clone request: the work is now running as a task.
#[derive(Debug, Serialize)]
pub struct CloneRepoSuccess {
    /// Id of the `TaskKind::GitClone` task running the clone. The frontend
    /// watches this in the tasks store to know when the repo is ready.
    pub task_id: TaskId,
    /// Absolute path the clone is landing in. Computed during validation,
    /// so it is known before the clone finishes; the frontend hands it to
    /// `open_project` once the task succeeds.
    pub path: String,
    /// Final folder name (basename of `path`). Convenient for toast
    /// messages so the FE does not have to re-parse the path.
    pub name: String,
}

/// A clone request that passed validation. Carries what the spawn needs.
#[derive(Debug)]
pub(crate) struct ValidatedClone {
    /// Absolute path the clone will create.
    pub(crate) target: PathBuf,
    /// Basename of `target`.
    pub(crate) name: String,
}

/// Tagged error so the FE can highlight which pipeline step failed.
#[derive(Debug, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum CloneRepoError {
    /// The clone URL was empty or did not match any of the shapes we
    /// recognise (`https://`, `http://`, `ssh://`, `git@host:path`,
    /// or a local path / file URL).
    InvalidUrl {
        /// Human-readable reason — used verbatim by the dialog banner.
        message: String,
    },
    /// The chosen parent directory does not exist or is not a directory.
    InvalidDestination {
        /// Human-readable reason — used verbatim by the dialog banner.
        message: String,
    },
    /// `<parent_dir>/<derived_name>` already exists. We refuse to overwrite
    /// it so the user does not accidentally clobber an existing checkout.
    DestinationExists {
        /// The full path that already exists. Surfaced to the user so
        /// they can either delete it or pick a different parent.
        path: String,
    },
}

/// Pure validation (no `AppState`, no I/O beyond `stat`). [`clone_repo`]
/// calls this before spawning the task; tests drive it directly so no IPC
/// is required.
pub(crate) fn validate_clone_request(
    opts: &CloneRepoOptions,
) -> Result<ValidatedClone, CloneRepoError> {
    let url = opts.url.trim();
    if url.is_empty() {
        return Err(CloneRepoError::InvalidUrl {
            message: "URL is empty".into(),
        });
    }
    // Reject any control character (CR/LF, NUL, …) or whitespace inside the
    // URL — `git clone` accepts these silently and they are the standard
    // exfiltration vectors for CVE-class clone attacks (e.g. embedded
    // newline that flips a follow-up `git config` line). The legitimate
    // clone URL space contains none of them.
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(CloneRepoError::InvalidUrl {
            message: format!("'{url}' contains whitespace or control characters"),
        });
    }
    if !looks_like_clone_url(url) {
        return Err(CloneRepoError::InvalidUrl {
            message: format!(
                "'{url}' does not look like a clone URL (expected https://, http://, ssh://, git@host:path, or a local path)"
            ),
        });
    }

    let parent = Path::new(opts.parent_dir.trim());
    if !parent.is_dir() {
        return Err(CloneRepoError::InvalidDestination {
            message: format!("'{}' is not a directory", parent.display()),
        });
    }

    let name = derive_repo_name(url).ok_or_else(|| CloneRepoError::InvalidUrl {
        message: format!("could not derive a repository name from '{url}'"),
    })?;
    let target = parent.join(&name);

    if target.exists() {
        return Err(CloneRepoError::DestinationExists {
            path: target.to_string_lossy().into_owned(),
        });
    }

    Ok(ValidatedClone { target, name })
}

/// Returns true iff `url` matches one of the prefixes the FE also
/// validates against. Keep these two lists in sync when adding new
/// shapes (`InitRepoDialog` uses the same set).
fn looks_like_clone_url(url: &str) -> bool {
    const PREFIXES: &[&str] = &["https://", "http://", "ssh://", "git://", "file://", "git@"];
    if PREFIXES.iter().any(|p| url.starts_with(p)) {
        return true;
    }
    // Local path forms — same set the InitRepoDialog accepts.
    url.starts_with('/') || url.starts_with("./") || url.starts_with("../")
}

/// Pulls the would-be folder name out of a clone URL the way `git clone`
/// itself does — last path segment with a trailing `.git` stripped.
///
/// Handles both URL-style inputs (`https://host/owner/repo.git`) and
/// SCP-style SSH (`git@host:owner/repo.git`).
fn derive_repo_name(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    // SCP-style: split on the first ':' so we treat `git@host:owner/repo.git`
    // as path `owner/repo.git`.
    let path = trimmed
        .rsplit_once(':')
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let last = path.rsplit('/').next()?;
    let stem = last.strip_suffix(".git").unwrap_or(last);
    let stem = stem.trim();
    if stem.is_empty() {
        None
    } else {
        Some(stem.into())
    }
}

/// Tauri command. Validates, then spawns the clone as a task and returns
/// immediately with its id.
///
/// Validation keeps its typed [`CloneRepoError`] (so its tests can match on
/// the failing step); the command boundary folds it into the shared
/// [`IpcError`] envelope, mapping each step to a stable `code`
/// (`invalid_url`, `invalid_destination`, `destination_exists`). A failure of
/// the clone itself is not an `IpcError` at all any more — it surfaces as a
/// failed task, with git's stderr in the task's output.
#[tauri::command]
// `skip_all`: `CloneRepoOptions` holds the source URL, which names a
// possibly-private repo. The destination is enough to follow the flow.
#[tracing::instrument(skip_all, fields(parent_dir = %options.parent_dir), name = "cmd::clone_repo")]
pub async fn clone_repo(
    options: CloneRepoOptions,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<CloneRepoSuccess, IpcError> {
    spawn_clone(&task_manager, &options)
        .await
        .map_err(IpcError::from)
}

/// Validate, then hand the clone to `task_manager`. Split out of
/// [`clone_repo`] so tests can drive the real spawn path without
/// constructing a Tauri `State`.
pub(crate) async fn spawn_clone(
    task_manager: &Arc<TaskManager>,
    options: &CloneRepoOptions,
) -> Result<CloneRepoSuccess, CloneRepoError> {
    let validated = validate_clone_request(options)?;

    let url = options.url.trim().to_string();
    let target = validated.target.to_string_lossy().into_owned();
    // Validation already established that this is an existing directory.
    let cwd = Path::new(options.parent_dir.trim());

    // The `--` separator stops `git` from interpreting a URL that begins with
    // `--` (or any unknown clone-url shape we add later) as a CLI flag. Belt-
    // and-suspenders next to `looks_like_clone_url`.
    let task_id = task_manager
        .spawn_with_options(SpawnOptions {
            label: format!("Clone {}", validated.name),
            command: "git",
            args: &["clone", "--", &url, &target],
            cwd,
            cancellable: true,
            kind: TaskKind::GitClone,
            stdin: None,
        })
        .await;

    Ok(CloneRepoSuccess {
        task_id,
        path: target,
        name: validated.name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn rejects_url_with_newline() {
        let opts = CloneRepoOptions {
            url: "https://example.com/repo.git\nrm -rf /".into(),
            parent_dir: ".".into(),
        };
        let err = validate_clone_request(&opts).unwrap_err();
        assert!(
            matches!(err, CloneRepoError::InvalidUrl { ref message } if message.contains("control")),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_url_with_embedded_space() {
        let opts = CloneRepoOptions {
            url: "https://example.com/repo .git".into(),
            parent_dir: ".".into(),
        };
        let err = validate_clone_request(&opts).unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidUrl { .. }));
    }

    #[test]
    fn rejects_url_with_nul_byte() {
        let opts = CloneRepoOptions {
            url: "https://example.com/repo\u{0}/x.git".into(),
            parent_dir: ".".into(),
        };
        let err = validate_clone_request(&opts).unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidUrl { .. }));
    }

    #[test]
    fn derive_name_handles_https() {
        assert_eq!(
            derive_repo_name("https://github.com/me/repo.git").as_deref(),
            Some("repo")
        );
        assert_eq!(
            derive_repo_name("https://github.com/me/repo").as_deref(),
            Some("repo")
        );
        assert_eq!(
            derive_repo_name("https://gitlab.com/group/sub/proj.git").as_deref(),
            Some("proj"),
        );
    }

    #[test]
    fn derive_name_handles_scp_style_ssh() {
        assert_eq!(
            derive_repo_name("git@github.com:me/repo.git").as_deref(),
            Some("repo"),
        );
        assert_eq!(
            derive_repo_name("git@gitlab.com:group/sub/proj").as_deref(),
            Some("proj"),
        );
    }

    #[test]
    fn derive_name_handles_ssh_url() {
        assert_eq!(
            derive_repo_name("ssh://git@github.com/me/repo.git").as_deref(),
            Some("repo"),
        );
    }

    #[test]
    fn derive_name_handles_trailing_slash() {
        assert_eq!(
            derive_repo_name("https://github.com/me/repo/").as_deref(),
            Some("repo"),
        );
    }

    #[test]
    fn derive_name_returns_none_when_basename_is_empty() {
        assert_eq!(derive_repo_name(""), None);
        assert_eq!(derive_repo_name(".git"), None);
    }

    #[test]
    fn looks_like_clone_url_accepts_known_prefixes() {
        for ok in [
            "https://github.com/x/y.git",
            "http://example.com/x.git",
            "ssh://git@host/x.git",
            "git://host/x.git",
            "file:///tmp/x.git",
            "git@github.com:x/y.git",
            "/srv/git/x.git",
            "./x",
            "../x",
        ] {
            assert!(looks_like_clone_url(ok), "expected ok: {ok}");
        }
    }

    #[test]
    fn looks_like_clone_url_rejects_garbage() {
        assert!(!looks_like_clone_url(""));
        assert!(!looks_like_clone_url("not-a-url"));
        assert!(!looks_like_clone_url("ftp://example.com/x"));
    }

    #[test]
    fn pipeline_rejects_empty_url() {
        let err = validate_clone_request(&CloneRepoOptions {
            url: "  ".into(),
            parent_dir: ".".into(),
        })
        .unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidUrl { .. }));
    }

    #[test]
    fn pipeline_rejects_unrecognised_url_shape() {
        let err = validate_clone_request(&CloneRepoOptions {
            url: "ftp://example.com/x".into(),
            parent_dir: ".".into(),
        })
        .unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidUrl { .. }));
    }

    #[test]
    fn pipeline_rejects_missing_parent_dir() {
        let err = validate_clone_request(&CloneRepoOptions {
            url: "https://example.com/x.git".into(),
            parent_dir: "/definitely/not/a/real/path/here".into(),
        })
        .unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidDestination { .. }));
    }

    #[test]
    fn validation_refuses_to_overwrite_existing_target() {
        let tmp = tempfile::tempdir().unwrap();
        // Pre-create the target subdir so the pipeline trips on its existence
        // check before it ever invokes `git`.
        std::fs::create_dir(tmp.path().join("repo")).unwrap();
        let err = validate_clone_request(&CloneRepoOptions {
            url: "https://example.com/me/repo.git".into(),
            parent_dir: tmp.path().to_string_lossy().into_owned(),
        })
        .unwrap_err();
        match err {
            CloneRepoError::DestinationExists { path } => {
                assert!(path.ends_with("repo"), "unexpected path: {path}");
            }
            other => panic!("expected DestinationExists, got {other:?}"),
        }
    }

    /// End-to-end smoke test: build a tiny bare repo on disk, then point the
    /// real spawn path at it via a `file://` URL. Avoids hitting the network.
    ///
    /// Drives `spawn_clone` + `wait_for_terminal` rather than running `git
    /// clone` directly, so it covers what production actually does: the args
    /// this module builds, executed by `TaskManager`.
    #[tokio::test]
    async fn spawn_clone_clones_a_local_bare_repo() {
        use std::path::PathBuf;
        let src = tempfile::tempdir().unwrap();
        let bare = src.path().join("origin.git");
        let init_status = Command::new("git")
            .args(["init", "--bare", "--initial-branch=main"])
            .arg(&bare)
            .status()
            .unwrap();
        assert!(init_status.success(), "git init --bare failed");

        // Seed the bare repo with one commit so `git clone` has something
        // to fetch and will leave a valid working tree.
        let work = tempfile::tempdir().unwrap();
        run_git(&work, &["init", "--initial-branch=main"]);
        run_git(&work, &["config", "user.email", "test@example.com"]);
        run_git(&work, &["config", "user.name", "Test"]);
        std::fs::write(work.path().join("README.md"), "hi\n").unwrap();
        run_git(&work, &["add", "README.md"]);
        run_git(&work, &["commit", "-m", "init"]);
        run_git(&work, &["remote", "add", "origin", &bare.to_string_lossy()]);
        run_git(&work, &["push", "origin", "main"]);

        let dest = tempfile::tempdir().unwrap();
        // Build a well-formed file URL on every platform: Windows paths use
        // backslashes and a drive letter, which would otherwise produce a
        // malformed URL the pipeline's name-derivation can't split.
        let bare_url_path = bare.to_string_lossy().replace('\\', "/");
        let bare_url = if bare_url_path.starts_with('/') {
            format!("file://{bare_url_path}")
        } else {
            format!("file:///{bare_url_path}")
        };
        let manager = Arc::new(TaskManager::new(Arc::new(NoopSink)));
        let success = spawn_clone(
            &manager,
            &CloneRepoOptions {
                url: bare_url,
                parent_dir: dest.path().to_string_lossy().into_owned(),
            },
        )
        .await
        .unwrap();
        assert_eq!(success.name, "origin");

        let status = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            manager.wait_for_terminal(success.task_id),
        )
        .await
        .expect("clone task should finish promptly")
        .expect("task should be in the registry");
        assert!(
            matches!(status, task_runner::TaskStatus::Completed),
            "expected Completed, got {status:?}"
        );

        let cloned: PathBuf = success.path.into();
        assert!(cloned.join(".git").is_dir());
        assert!(cloned.join("README.md").is_file());
    }

    /// Minimal sink: the clone path emits through `TaskEmitter`, not this,
    /// so the test only needs the trait satisfied.
    struct NoopSink;

    #[async_trait::async_trait]
    impl task_runner::TaskEventSink for NoopSink {
        async fn on_task_started(&self, _info: task_runner::TaskInfo) {}
        async fn on_task_output(&self, _task_id: TaskId, _line: task_runner::OutputLine) {}
        async fn on_task_completed(&self, _info: task_runner::TaskInfo) {}
        async fn on_task_failed(&self, _info: task_runner::TaskInfo) {}
        async fn on_task_cancelled(&self, _info: task_runner::TaskInfo) {}
    }

    fn run_git(dir: &tempfile::TempDir, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(dir.path())
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed in {:?}", dir.path());
    }
}
