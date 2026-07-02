//! Structured IPC error envelope (spec 05, Phase 3).
//!
//! Commands migrated off `Result<_, String>` return [`IpcError`] instead — a
//! `{ code, message }` pair that Tauri serialises to the JS rejection value.
//! The stable snake_case `code` lets the frontend branch (auth vs. not-a-repo
//! vs. non-fast-forward) instead of pattern-matching free text, and `message`
//! carries the human-readable detail. `From` impls fold the crate's existing
//! typed errors into a code so a migration is a one-line `.map_err(IpcError::from)`
//! or a bare `?`.

use serde::Serialize;

use crate::commands::{CloneRepoError, InitRepoError, OpenProjectError};

/// A structured error returned across the IPC boundary.
#[derive(Debug, Clone, Serialize)]
pub struct IpcError {
    /// Stable machine-readable code (snake_case), e.g. `"auth_required"`,
    /// `"repo_not_found"`, `"not_fast_forward"`. The frontend switches on this.
    pub code: &'static str,
    /// Human-readable detail, suitable for a toast body.
    pub message: String,
}

impl IpcError {
    /// Construct an [`IpcError`] from a static code and any string-like message.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for IpcError {}

/// Fallback for the `.map_err(|e| e.to_string())?` sites a partially-migrated
/// command body still carries: a plain `String` flows into the generic
/// `"error"` code so the function compiles without rewriting every arm.
impl From<String> for IpcError {
    fn from(message: String) -> Self {
        Self {
            code: "error",
            message,
        }
    }
}

impl From<git_engine::GitError> for IpcError {
    fn from(err: git_engine::GitError) -> Self {
        use git_engine::GitError as G;
        let code = match &err {
            // libgit2 carries a finer code we can lift for the two cases the
            // frontend wants to branch on; everything else stays generic.
            G::Git(e) => match e.code() {
                git2::ErrorCode::Auth => "auth_required",
                git2::ErrorCode::NotFastForward => "not_fast_forward",
                _ => "git",
            },
            G::RepoNotFound(_) => "repo_not_found",
            G::CliError(_) => "cli_error",
            G::Io(_) => "io_error",
            G::Binary => "binary_file",
            G::FileTooLarge { .. } => "file_too_large",
            G::InvalidPath(_) => "invalid_path",
            G::InvalidArgument(_) => "invalid_argument",
        };
        Self {
            code,
            message: err.to_string(),
        }
    }
}

impl From<CloneRepoError> for IpcError {
    fn from(err: CloneRepoError) -> Self {
        match err {
            CloneRepoError::InvalidUrl { message } => Self::new("invalid_url", message),
            CloneRepoError::InvalidDestination { message } => {
                Self::new("invalid_destination", message)
            }
            // The path that already exists is the actionable detail — carry it
            // as the message so the dialog can echo it.
            CloneRepoError::DestinationExists { path } => Self::new("destination_exists", path),
            CloneRepoError::Clone { message } => Self::new("clone_failed", message),
        }
    }
}

impl From<OpenProjectError> for IpcError {
    fn from(err: OpenProjectError) -> Self {
        match err {
            // The attempted path is the actionable detail — carry it as the
            // message so the frontend can seed the "init repo here?" dialog.
            OpenProjectError::NotARepo { path } => Self::new("not_a_repo", path),
            OpenProjectError::Other { message } => Self::new("open_failed", message),
        }
    }
}

impl From<InitRepoError> for IpcError {
    fn from(err: InitRepoError) -> Self {
        match err {
            InitRepoError::Init { message } => Self::new("init_failed", message),
            InitRepoError::Gitignore { message } => Self::new("gitignore_failed", message),
            InitRepoError::Commit { message } => Self::new("commit_failed", message),
            InitRepoError::CreateRemote { provider, message } => {
                Self::new("create_remote_failed", format!("{provider}: {message}"))
            }
            InitRepoError::AddOrigin { message } => Self::new("add_origin_failed", message),
            InitRepoError::Push { message } => Self::new("push_failed", message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialises_to_code_and_message() {
        let e = IpcError::new("auth_required", "authentication failed");
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            json!({ "code": "auth_required", "message": "authentication failed" }),
        );
    }

    #[test]
    fn from_string_uses_generic_code() {
        let e: IpcError = "boom".to_string().into();
        assert_eq!(e.code, "error");
        assert_eq!(e.message, "boom");
    }

    #[test]
    fn from_git_error_maps_variants() {
        assert_eq!(
            IpcError::from(git_engine::GitError::RepoNotFound("/x".into())).code,
            "repo_not_found",
        );
        assert_eq!(
            IpcError::from(git_engine::GitError::Binary).code,
            "binary_file",
        );
        assert_eq!(
            IpcError::from(git_engine::GitError::FileTooLarge { size: 10 }).code,
            "file_too_large",
        );
    }

    #[test]
    fn from_clone_error_maps_step_to_code() {
        assert_eq!(
            IpcError::from(CloneRepoError::InvalidUrl {
                message: "bad".into()
            })
            .code,
            "invalid_url",
        );
        let dest = IpcError::from(CloneRepoError::DestinationExists {
            path: "/tmp/x".into(),
        });
        assert_eq!(dest.code, "destination_exists");
        assert_eq!(dest.message, "/tmp/x");
        assert_eq!(
            IpcError::from(CloneRepoError::Clone {
                message: "net".into()
            })
            .code,
            "clone_failed",
        );
    }

    #[test]
    fn from_open_project_error_maps_kind_to_code() {
        let not_a_repo = IpcError::from(OpenProjectError::NotARepo {
            path: "/tmp/foo".into(),
        });
        assert_eq!(not_a_repo.code, "not_a_repo");
        assert_eq!(not_a_repo.message, "/tmp/foo");
        assert_eq!(
            IpcError::from(OpenProjectError::Other {
                message: "boom".into()
            })
            .code,
            "open_failed",
        );
    }

    #[test]
    fn from_init_error_maps_step_to_code() {
        assert_eq!(
            IpcError::from(InitRepoError::Push {
                message: "rejected".into()
            })
            .code,
            "push_failed",
        );
        assert_eq!(
            IpcError::from(InitRepoError::CreateRemote {
                provider: "GitHub".into(),
                message: "taken".into(),
            })
            .message,
            "GitHub: taken",
        );
    }
}
