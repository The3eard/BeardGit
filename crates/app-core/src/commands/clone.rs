//! `clone_repo` — clones a remote git repository into a local parent
//! directory and returns the absolute path of the resulting working tree.
//!
//! Mirrors the shape of [`super::init`]: a small typed payload, a pure
//! pipeline that does the actual work (`run_clone_pipeline`), and a thin
//! Tauri wrapper that the frontend invokes from `CloneRepoDialog`.
//!
//! The pipeline shells out to `git clone <url> <target>` so cred-helpers
//! (`gh auth`, `glab auth`, `osxkeychain`, …) and SSH agents Just Work the
//! same way they do everywhere else in BeardGit. We intentionally do not
//! use `git2`'s built-in clone here: libgit2 cannot reuse the user's
//! configured credential helpers, which would give us a worse UX than the
//! status quo (where the user runs `git clone` in a terminal).
//!
//! Each step is independently fallible. The error enum is tagged so the
//! dialog banner can branch on the failure mode without parsing free
//! text — same convention as [`super::init::InitRepoError`].

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Options accepted by [`clone_repo`] (and `run_clone_pipeline`).
#[derive(Debug, Deserialize)]
pub struct CloneRepoOptions {
    /// Clone URL — HTTPS, SSH, or `git@` shorthand.
    pub url: String,
    /// Absolute path to the *parent* folder where the repo should land.
    /// The pipeline derives the final folder name from `url` and creates
    /// it as a subdirectory of `parent_dir`.
    pub parent_dir: String,
}

/// Successful pipeline outcome.
#[derive(Debug, Serialize)]
pub struct CloneRepoSuccess {
    /// Absolute path of the freshly cloned working tree. The frontend
    /// hands this straight back to `open_project` to mount it as a tab.
    pub path: String,
    /// Final folder name (basename of `path`). Convenient for toast
    /// messages so the FE does not have to re-parse the path.
    pub name: String,
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
    /// `git clone` itself failed (network error, auth rejected, repository
    /// not found, …). `message` carries the captured stderr.
    Clone {
        /// stderr from the failing `git clone` invocation, trimmed.
        message: String,
    },
}

/// Pure pipeline (no `AppState`). [`clone_repo`] wraps this with the
/// `#[tauri::command]` attribute; tests drive it directly so no IPC is
/// required.
pub(crate) fn run_clone_pipeline(
    opts: &CloneRepoOptions,
) -> Result<CloneRepoSuccess, CloneRepoError> {
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

    // The `--` separator stops `git` from interpreting a URL that begins with
    // `--` (or any unknown clone-url shape we add later) as a CLI flag. Belt-
    // and-suspenders next to `looks_like_clone_url`.
    let output = Command::new("git")
        .arg("clone")
        .arg("--")
        .arg(url)
        .arg(&target)
        .output()
        .map_err(|e| CloneRepoError::Clone {
            message: format!("failed to spawn git: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CloneRepoError::Clone {
            message: if stderr.is_empty() {
                format!("git clone exited with status {}", output.status)
            } else {
                stderr
            },
        });
    }

    Ok(CloneRepoSuccess {
        path: target.to_string_lossy().into_owned(),
        name,
    })
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

/// Tauri command. Thin wrapper around [`run_clone_pipeline`] — kept
/// trivial so all the testable logic lives in the pure function above.
#[tauri::command]
#[tracing::instrument(name = "cmd::clone_repo")]
pub fn clone_repo(options: CloneRepoOptions) -> Result<CloneRepoSuccess, CloneRepoError> {
    run_clone_pipeline(&options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_url_with_newline() {
        let opts = CloneRepoOptions {
            url: "https://example.com/repo.git\nrm -rf /".into(),
            parent_dir: ".".into(),
        };
        let err = run_clone_pipeline(&opts).unwrap_err();
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
        let err = run_clone_pipeline(&opts).unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidUrl { .. }));
    }

    #[test]
    fn rejects_url_with_nul_byte() {
        let opts = CloneRepoOptions {
            url: "https://example.com/repo\u{0}/x.git".into(),
            parent_dir: ".".into(),
        };
        let err = run_clone_pipeline(&opts).unwrap_err();
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
        let err = run_clone_pipeline(&CloneRepoOptions {
            url: "  ".into(),
            parent_dir: ".".into(),
        })
        .unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidUrl { .. }));
    }

    #[test]
    fn pipeline_rejects_unrecognised_url_shape() {
        let err = run_clone_pipeline(&CloneRepoOptions {
            url: "ftp://example.com/x".into(),
            parent_dir: ".".into(),
        })
        .unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidUrl { .. }));
    }

    #[test]
    fn pipeline_rejects_missing_parent_dir() {
        let err = run_clone_pipeline(&CloneRepoOptions {
            url: "https://example.com/x.git".into(),
            parent_dir: "/definitely/not/a/real/path/here".into(),
        })
        .unwrap_err();
        assert!(matches!(err, CloneRepoError::InvalidDestination { .. }));
    }

    #[test]
    fn pipeline_refuses_to_overwrite_existing_target() {
        let tmp = tempfile::tempdir().unwrap();
        // Pre-create the target subdir so the pipeline trips on its existence
        // check before it ever invokes `git`.
        std::fs::create_dir(tmp.path().join("repo")).unwrap();
        let err = run_clone_pipeline(&CloneRepoOptions {
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

    #[test]
    fn pipeline_clones_a_local_bare_repo() {
        use std::path::PathBuf;
        // End-to-end smoke test: build a tiny bare repo on disk, then point
        // the pipeline at it via a `file://` URL. Avoids hitting the network.
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
        let success = run_clone_pipeline(&CloneRepoOptions {
            url: format!("file://{}", bare.display()),
            parent_dir: dest.path().to_string_lossy().into_owned(),
        })
        .unwrap();
        assert_eq!(success.name, "origin");
        let cloned: PathBuf = success.path.into();
        assert!(cloned.join(".git").is_dir());
        assert!(cloned.join("README.md").is_file());
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
