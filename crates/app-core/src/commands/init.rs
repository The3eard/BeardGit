//! `init_repo` — the pipeline that backs the InitRepoDialog.
//!
//! Given a folder that may not yet be a git repository, this command runs:
//!   1. `git init` with `initial_branch=main`
//!   2. write `.gitignore` if requested AND not already present
//!   3. `git add -A` + commit "Initial commit" if requested
//!   4. either create a new repo on the requested forge provider, OR wire an
//!      already-existing remote URL (`RemoteSpec::UseExisting`); then run
//!      `git remote add origin`
//!   5. `git push origin <initial_branch>` (when `push_after`)
//!
//! Each step is independently fallible. Partial success is preserved —
//! we never roll back a successful step. The Tauri layer (`init_repo`)
//! is a thin wrapper that resolves the forge provider; all business logic
//! lives in [`run_init_pipeline`] so it can be unit-tested without
//! `AppState`.

use forge_provider::{CreateRepoInput, ForgeError, ForgeKind};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::state::AppState;

/// Options accepted by [`init_repo`] (and `run_init_pipeline`).
#[derive(Debug, Deserialize)]
pub struct InitRepoOptions {
    /// Absolute path to the folder to initialise.
    pub path: String,
    /// Branch to set as the initial HEAD (typically "main").
    pub initial_branch: String,
    /// `.gitignore` contents to write *if* none exists already. `None` skips.
    pub gitignore: Option<String>,
    /// Stage all files and create the "Initial commit" when true.
    pub initial_commit: bool,
    /// Optional remote-creation step.
    pub remote: Option<RemoteSpec>,
}

/// Subset of [`InitRepoOptions`] describing what to do with a remote.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RemoteSpec {
    /// Create a new repo on the given provider and set it as `origin`.
    Create {
        /// Index into [`AppState::providers`].
        provider_index: usize,
        /// Repo name on the forge.
        name: String,
        /// Whether the new repo should be private.
        private: bool,
        /// Push the initial branch immediately after creation.
        push_after: bool,
    },
    /// Wire an already-existing remote URL as origin and (optionally) push.
    /// Skips the forge provider entirely; works without any provider
    /// connected.
    UseExisting {
        /// Full clone URL — HTTPS, SSH, or git@ — of the remote to use.
        url: String,
        /// Push the initial branch immediately after wiring origin.
        push_after: bool,
    },
}

/// Successful pipeline outcome.
#[derive(Debug, Serialize)]
pub struct InitRepoSuccess {
    /// Browser-facing URL of the new remote, when one was created.
    pub web_url: Option<String>,
}

/// Tagged error so the FE can highlight which pipeline step failed.
#[derive(Debug, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum InitRepoError {
    /// `git init` failed (e.g. permission denied, path is a file, etc.).
    Init {
        /// Human-readable message extracted from the underlying error.
        message: String,
    },
    /// Writing the `.gitignore` file failed (filesystem error).
    Gitignore {
        /// Human-readable message extracted from the underlying error.
        message: String,
    },
    /// Staging or committing the initial snapshot failed (often missing
    /// `user.name` / `user.email`).
    Commit {
        /// Human-readable message extracted from the underlying error.
        message: String,
    },
    /// The forge provider rejected the create-repo call (network error,
    /// name taken, auth missing, …).
    CreateRemote {
        /// Human-readable provider label ("GitHub", "GitLab"). Used by
        /// the dialog banner.
        provider: String,
        /// Reason supplied by the provider.
        message: String,
    },
    /// `git remote add origin <url>` failed locally.
    AddOrigin {
        /// Human-readable message extracted from the underlying error.
        message: String,
    },
    /// `git push origin <branch>` failed.
    Push {
        /// Human-readable message extracted from the underlying error.
        message: String,
    },
}

/// Pure pipeline (no AppState). [`init_repo`] wraps this to inject the
/// forge provider; tests drive it directly with a `MockProvider`.
pub(crate) fn run_init_pipeline(
    opts: &InitRepoOptions,
    provider: Option<&dyn forge_provider::ForgeProvider>,
) -> Result<InitRepoSuccess, InitRepoError> {
    // Step 1: git init with initial_branch.
    let mut init_opts = git2::RepositoryInitOptions::new();
    init_opts
        .initial_head(&opts.initial_branch)
        .mkdir(true)
        .no_reinit(false);
    git2::Repository::init_opts(&opts.path, &init_opts).map_err(|e| InitRepoError::Init {
        message: e.to_string(),
    })?;

    // Step 2: write .gitignore if requested AND not already present.
    if let Some(content) = &opts.gitignore {
        let target = std::path::Path::new(&opts.path).join(".gitignore");
        if !target.exists()
            && let Err(e) = std::fs::write(&target, content)
        {
            return Err(InitRepoError::Gitignore {
                message: e.to_string(),
            });
        }
    }

    // Step 3: stage all + commit "Initial commit".
    if opts.initial_commit {
        let repo = git_engine::Repository::open(&opts.path).map_err(|e| InitRepoError::Commit {
            message: e.to_string(),
        })?;
        repo.stage_all().map_err(|e| InitRepoError::Commit {
            message: e.to_string(),
        })?;
        repo.create_commit("Initial commit")
            .map_err(|e| InitRepoError::Commit {
                message: e.to_string(),
            })?;
    }

    // Step 4 + 5 + 6: create remote, add origin, push.
    match &opts.remote {
        None => {}
        Some(RemoteSpec::Create {
            name,
            private,
            push_after,
            ..
        }) => {
            let provider = provider.ok_or_else(|| InitRepoError::CreateRemote {
                provider: "unknown".into(),
                message: "no provider supplied".into(),
            })?;
            let provider_label = match provider.kind() {
                ForgeKind::GitHub => "GitHub",
                ForgeKind::GitLab => "GitLab",
            };
            let created = provider
                .create_repo(CreateRepoInput {
                    name: name.clone(),
                    private: *private,
                })
                .map_err(|e| InitRepoError::CreateRemote {
                    provider: provider_label.into(),
                    message: forge_error_to_message(e),
                })?;
            let repo =
                git2::Repository::open(&opts.path).map_err(|e| InitRepoError::AddOrigin {
                    message: e.to_string(),
                })?;
            repo.remote("origin", &created.clone_url)
                .map_err(|e| InitRepoError::AddOrigin {
                    message: e.to_string(),
                })?;
            if *push_after {
                push_initial(&opts.path, &opts.initial_branch)?;
            }
            return Ok(InitRepoSuccess {
                web_url: Some(created.web_url),
            });
        }
        Some(RemoteSpec::UseExisting { url, push_after }) => {
            let trimmed = url.trim();
            let repo =
                git2::Repository::open(&opts.path).map_err(|e| InitRepoError::AddOrigin {
                    message: e.to_string(),
                })?;
            repo.remote("origin", trimmed)
                .map_err(|e| InitRepoError::AddOrigin {
                    message: e.to_string(),
                })?;
            if *push_after {
                push_initial(&opts.path, &opts.initial_branch)?;
            }
            return Ok(InitRepoSuccess { web_url: None });
        }
    }

    Ok(InitRepoSuccess { web_url: None })
}

fn forge_error_to_message(e: ForgeError) -> String {
    match e {
        ForgeError::NameTaken => "repository name is already taken".into(),
        ForgeError::NotSupported => "this provider doesn't support creating repos".into(),
        other => other.to_string(),
    }
}

fn push_initial(path: &str, branch: &str) -> Result<(), InitRepoError> {
    let repo = git_engine::Repository::open(path).map_err(|e| InitRepoError::Push {
        message: e.to_string(),
    })?;
    let result = repo
        .push_remote("origin", branch, false)
        .map_err(|e| InitRepoError::Push {
            message: e.to_string(),
        })?;
    if result.success {
        Ok(())
    } else {
        Err(InitRepoError::Push {
            message: result.stderr,
        })
    }
}

/// Tauri command. Resolves the forge provider for the requested index
/// (when `remote = Create`) and delegates to [`run_init_pipeline`].
#[tauri::command]
#[tracing::instrument(skip(state, _app), name = "cmd::init_repo")]
pub fn init_repo(
    options: InitRepoOptions,
    state: State<'_, AppState>,
    _app: AppHandle,
) -> Result<InitRepoSuccess, InitRepoError> {
    let provider_arc = match &options.remote {
        Some(RemoteSpec::Create { provider_index, .. }) => Some(
            super::helpers::build_forge_provider_for_index(&state, *provider_index).map_err(
                |m| InitRepoError::CreateRemote {
                    provider: "unknown".into(),
                    message: m,
                },
            )?,
        ),
        Some(RemoteSpec::UseExisting { .. }) => None,
        None => None,
    };
    let provider_ref: Option<&dyn forge_provider::ForgeProvider> = provider_arc.as_deref();
    run_init_pipeline(&options, provider_ref)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_provider::{ForgeKind, mock::MockProvider};

    /// Pre-create the repo and configure a stable identity in its config so
    /// the pipeline's `create_commit` step can sign the commit. Call this
    /// instead of mutating process env vars (which races across tests).
    ///
    /// The pipeline's `init_opts` with `no_reinit(false)` will reopen the
    /// existing repo without rewriting `.git/config`, so the user.name /
    /// user.email written here survive into the commit step.
    fn pre_init_with_identity(path: &str, branch: &str) {
        let repo = git2::Repository::init_opts(
            path,
            git2::RepositoryInitOptions::new()
                .initial_head(branch)
                .mkdir(true),
        )
        .unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
    }

    fn opts(path: &str) -> InitRepoOptions {
        InitRepoOptions {
            path: path.into(),
            initial_branch: "main".into(),
            gitignore: None,
            initial_commit: false,
            remote: None,
        }
    }

    #[test]
    fn init_only_creates_repo_on_main() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        run_init_pipeline(&opts(&path), None).unwrap();
        let repo = git2::Repository::open(&path).unwrap();
        let head_ref = std::fs::read_to_string(repo.path().join("HEAD")).unwrap();
        assert!(head_ref.contains("refs/heads/main"));
    }

    #[test]
    fn writes_gitignore_when_provided_and_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let mut o = opts(&path);
        o.gitignore = Some("*.log\n".into());
        run_init_pipeline(&o, None).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "*.log\n");
    }

    #[test]
    fn skips_gitignore_when_already_present() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        std::fs::write(tmp.path().join(".gitignore"), "existing\n").unwrap();
        let mut o = opts(&path);
        o.gitignore = Some("REPLACED\n".into());
        run_init_pipeline(&o, None).unwrap();
        let content = std::fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
        assert_eq!(content, "existing\n");
    }

    #[test]
    fn initial_commit_stages_all_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("hello.txt"), "hi\n").unwrap();
        std::fs::write(tmp.path().join("nested.toml"), "x = 1\n").unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        pre_init_with_identity(&path, "main");
        let mut o = opts(&path);
        o.initial_commit = true;
        run_init_pipeline(&o, None).unwrap_or_else(|e| panic!("pipeline failed: {e:?}"));
        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap();
        let tree = head.peel_to_tree().unwrap();
        let names: Vec<_> = tree
            .iter()
            .map(|e| e.name().unwrap_or("").to_string())
            .collect();
        assert!(names.contains(&"hello.txt".to_string()));
        assert!(names.contains(&"nested.toml".to_string()));
    }

    #[test]
    fn create_remote_sets_origin_url() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "x").unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        pre_init_with_identity(&path, "main");
        let mut o = opts(&path);
        o.initial_commit = true;
        o.remote = Some(RemoteSpec::Create {
            provider_index: 0,
            name: "demo".into(),
            private: true,
            push_after: false,
        });
        let mock = MockProvider::new(ForgeKind::GitHub);
        let success = run_init_pipeline(&o, Some(&mock)).unwrap();
        assert!(success.web_url.unwrap().contains("demo"));
        let repo = git2::Repository::open(&path).unwrap();
        let remote = repo.find_remote("origin").unwrap();
        assert!(remote.url().unwrap().contains("demo"));
    }

    #[test]
    fn create_remote_failure_keeps_local_init() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "x").unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        pre_init_with_identity(&path, "main");
        let mut o = opts(&path);
        o.initial_commit = true;
        o.remote = Some(RemoteSpec::Create {
            provider_index: 0,
            name: "taken".into(),
            private: true,
            push_after: false,
        });
        let mock = MockProvider::new(ForgeKind::GitHub);
        mock.set_create_repo_error(ForgeError::NameTaken);
        let err = run_init_pipeline(&o, Some(&mock)).unwrap_err();
        assert!(matches!(err, InitRepoError::CreateRemote { .. }));
        // Local repo is still there.
        git2::Repository::open(&path).unwrap();
    }

    #[test]
    fn create_remote_failure_carries_provider_label() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "x").unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        pre_init_with_identity(&path, "main");
        let mut o = opts(&path);
        o.initial_commit = true;
        o.remote = Some(RemoteSpec::Create {
            provider_index: 0,
            name: "taken".into(),
            private: true,
            push_after: false,
        });
        let mock = MockProvider::new(ForgeKind::GitHub);
        mock.set_create_repo_error(ForgeError::NameTaken);
        let err = run_init_pipeline(&o, Some(&mock)).unwrap_err();
        match err {
            InitRepoError::CreateRemote {
                provider,
                message: _,
            } => {
                assert_eq!(provider, "GitHub");
            }
            other => panic!("expected CreateRemote, got {other:?}"),
        }
    }

    #[test]
    fn use_existing_remote_sets_origin() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "x").unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        pre_init_with_identity(&path, "main");
        let mut o = opts(&path);
        o.initial_commit = true;
        o.remote = Some(RemoteSpec::UseExisting {
            url: "https://example.test/me/repo.git".into(),
            push_after: false,
        });
        let success = run_init_pipeline(&o, None).unwrap();
        assert!(success.web_url.is_none());
        let repo = git2::Repository::open(&path).unwrap();
        let remote = repo.find_remote("origin").unwrap();
        assert_eq!(remote.url().unwrap(), "https://example.test/me/repo.git");
    }

    #[test]
    fn use_existing_remote_skips_provider_when_none_supplied() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        pre_init_with_identity(&path, "main");
        let mut o = opts(&path);
        o.remote = Some(RemoteSpec::UseExisting {
            url: "https://example.test/me/repo.git".into(),
            push_after: false,
        });
        // The mock provider must NOT be touched. Pass None to prove it.
        run_init_pipeline(&o, None).unwrap();
    }

    #[test]
    fn use_existing_remote_trims_whitespace() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        pre_init_with_identity(&path, "main");
        let mut o = opts(&path);
        o.remote = Some(RemoteSpec::UseExisting {
            url: "  https://example.test/me/repo.git  ".into(),
            push_after: false,
        });
        run_init_pipeline(&o, None).unwrap();
        let repo = git2::Repository::open(&path).unwrap();
        let remote = repo.find_remote("origin").unwrap();
        assert_eq!(remote.url().unwrap(), "https://example.test/me/repo.git");
    }
}
