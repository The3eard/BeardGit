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
    /// `"not_a_repo"`, `"not_fast_forward"`. The frontend switches on this.
    pub code: &'static str,
    /// Human-readable detail, suitable for a toast body.
    pub message: String,
}

/// Cap on the error detail written to the log.
///
/// The detail is usually our own prose, but for `cli_error` / `signing_failed`
/// it is raw stderr from git — or from a user's git hook, which can print
/// anything, including a diff. Capping bounds that exposure while keeping
/// enough of the message to identify the failure.
const LOG_DETAIL_MAX: usize = 300;

fn truncate_detail(detail: &str) -> String {
    match detail.char_indices().nth(LOG_DETAIL_MAX) {
        Some((idx, _)) => format!("{}… (truncated)", &detail[..idx]),
        None => detail.to_string(),
    }
}

impl IpcError {
    /// Construct an [`IpcError`] from a static code and any string-like message.
    ///
    /// Every construction path funnels through here — including the `From`
    /// impls below — so this is also the single place that logs IPC
    /// failures.
    ///
    /// **Coverage is all 311 registered commands** — the 281 in
    /// `commands/` plus `ai_commands`, `terminal_commands` and
    /// `task_commands`, which are registered from their own modules and
    /// were missed by a first count that only looked at `commands/`.
    ///
    /// It was 27 while the migration off `Result<_, String>` was in
    /// progress: a command still on the old signature failed silently, so
    /// the user got a toast and the log got nothing. That was the argument
    /// for finishing it rather than leaving it perpetually last, and it is
    /// why the hook lives here rather than in each command body.
    ///
    /// Use [`IpcError::expected`] for conditions that are routine rather
    /// than wrong — see its docs.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        let message = message.into();
        // `detail`, not `message`: tracing reserves `message` for the
        // event's own text, so a field by that name renders unlabelled.
        tracing::error!(
            code,
            detail = %truncate_detail(&message),
            "ipc command failed"
        );
        Self { code, message }
    }

    /// Build an [`IpcError`] **without** logging it.
    ///
    /// For conditions that are expected rather than wrong. "No active
    /// project" and "no repository open" are the shape behind most
    /// "nothing happened" reports and were deliberately logged at DEBUG,
    /// but the migration routed them through `new` — which meant every
    /// read command dispatched against a background tab (heavy state is
    /// `None` there, by the active-tab invariant) wrote an ERROR line.
    /// Logging every genuine failure is the point of this type; drowning
    /// it in routine ones is not.
    pub fn expected(code: &'static str, message: impl Into<String>) -> Self {
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
        Self::new("error", message)
    }
}

/// Same fallback for a string literal. Command bodies raise plenty of
/// `Err("no active project")`-shaped failures, and without this every one
/// of them needs a `.to_string()` before it can flow into the envelope.
impl From<&str> for IpcError {
    fn from(message: &str) -> Self {
        Self::new("error", message)
    }
}

// There is deliberately no `impl From<IpcError> for String`.
//
// One existed, to satisfy the `E: From<IpcError>` bound on
// `with_mutation_guard_async` for command bodies whose inner
// `spawn_blocking` closure worked in `String`. It cost more than it saved,
// in two ways.
//
// It made `IpcError: Into<String>` hold, so an
// `IpcError::new("internal", some_ipc_error)` compiled silently — dropping
// the original code and logging a second time under the wrong one.
// Nineteen callsites did exactly that.
//
// Worse, and only found later: those `String`-typed closures were
// *actively* flattening error codes. `map_err(|e| e.to_string())` on a
// `GitError` throws the variant away, and the trailing
// `.map_err(IpcError::from)` then rebuilt it as the generic `"error"`. 24
// functions across staging, worktree, remote, config, advanced and
// repository did that; they now let `?` carry the code.
//
// Be precise about what that bought, because the obvious claim is wrong.
// The codes those sites can actually emit are `not_a_repo` (from
// `Repository::open`) plus `git` / `cli_error` / `io_error` /
// `invalid_argument`, and all but the first are `@unmapped` in
// `errors.ts`, so they render as the raw message either way. The gain is
// log fidelity and an honest envelope, *not* better toast text. In
// particular `would_lose_changes` and `not_fully_merged` were never
// affected: their only producers are `delete_branch`, `checkout_branch`
// and `checkout_detached`, reached from `branch.rs`, which already used
// the correct idiom.
//
// One case did change the toast, in the wrong direction, and is worth
// remembering as the general hazard: `worktree.rs` in `git-engine` was
// using `GitError::RepoNotFound` as a catch-all for any failed `git
// worktree` invocation. Recovering the code turned "…already exists" into
// the mapped sentence "Not a git repository". Fixed at the source
// (`CliError`, like its neighbours) — but the lesson is that recovering a
// code is only an improvement if the code is *right*, and `errorCodeMessage`
// replaces the message rather than adding to it.
//
// The absence of this impl is narrower enforcement than it looks. It stops
// `IpcError → String → IpcError`: that no longer compiles, wherever a
// `String`-typed closure meets `with_mutation_guard_async` or
// `with_active_repo`. It does **not** stop `GitError → String →
// IpcError`, because `From<String> for IpcError` above still exists — and
// roughly 37 commands still do exactly that (`diff.rs` has 14, `tag.rs`,
// `stash.rs`, `file_editor.rs` and `graph.rs` three each, and so on). Not
// a blind sweep waiting to happen: each site's `GitError` variant has to
// be checked against `errorCodeMessage` first, for the reason above.
//
// So: do not add this impl back. If a closure needs to yield `IpcError`,
// let `?` do the work via the `From` impls below, and reach for
// `helpers::run_blocking` rather than a bare `spawn_blocking` so the
// command's tracing span survives the thread hop — moving `IpcError`
// construction into a closure moves its `tracing::error!` onto a pool
// thread, where `#[instrument(name = "cmd::…")]` is not in scope. If a
// probe's failure should not be logged, that is what `IpcError::expected`
// is for.

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
            // Unified with `open_project`'s `OpenProjectError::NotARepo`: both
            // "this path isn't a git repo" situations share one code.
            G::RepoNotFound(_) => "not_a_repo",
            G::CliError(_) => "cli_error",
            G::SigningFailed(_) => "signing_failed",
            G::Io(_) => "io_error",
            G::Binary => "binary_file",
            G::FileTooLarge { .. } => "file_too_large",
            G::InvalidPath(_) => "invalid_path",
            G::InvalidArgument(_) => "invalid_argument",
            G::WouldLoseChanges(_) => "would_lose_changes",
            G::NotFullyMerged(_) => "not_fully_merged",
            G::BranchAlreadyExists(_) => "branch_exists",
        };
        Self::new(code, err.to_string())
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
            "not_a_repo",
        );
        assert_eq!(
            IpcError::from(git_engine::GitError::Binary).code,
            "binary_file",
        );
        assert_eq!(
            IpcError::from(git_engine::GitError::WouldLoseChanges("dirty".into())).code,
            "would_lose_changes",
        );
        assert_eq!(
            IpcError::from(git_engine::GitError::NotFullyMerged("unmerged".into())).code,
            "not_fully_merged",
        );
        assert_eq!(
            IpcError::from(git_engine::GitError::BranchAlreadyExists("exists".into())).code,
            "branch_exists",
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
