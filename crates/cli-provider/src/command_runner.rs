//! Testable CLI subprocess seam used by the repo-config commands.
//!
//! The [`CommandRunner`] trait abstracts process execution so every
//! `gh` / `glab` invocation used by the repo-configuration feature can
//! be unit-tested without shelling out to a real binary. Two
//! implementations ship:
//!
//! - [`SystemRunner`] — shells out via `std::process::Command`. Used
//!   by the live Tauri commands.
//! - [`MockRunner`] — records every `(cmd, args, cwd)` tuple into a
//!   shared `Vec` and returns canned results keyed by `(cmd,
//!   args_prefix)`. Tests assert exact argv to guarantee the argument
//!   builder is shell-injection safe.
//!
//! The trait is intentionally synchronous so it composes cleanly with
//! `tokio::task::spawn_blocking` in the Tauri command dispatchers —
//! each individual CLI invocation is short, and the callers already
//! move off the async runtime to avoid blocking the event loop.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use thiserror::Error;

/// Output of a [`CommandRunner::run`] invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    /// Captured standard output as a UTF-8 string (lossy).
    pub stdout: String,
    /// Captured standard error as a UTF-8 string (lossy).
    pub stderr: String,
    /// Exit code reported by the child process; `-1` on signal termination.
    pub exit_code: i32,
}

/// Errors surfaced by [`CommandRunner::run`].
#[derive(Debug, Error)]
pub enum CliError {
    /// The CLI binary could not be located on disk or `PATH`.
    #[error("CLI binary not found: {0}")]
    NotFound(String),
    /// The command ran and returned a non-zero exit code.
    ///
    /// The field carries stdout + stderr so callers that only want the
    /// "failure text" can reach into either stream.
    #[error("CLI command failed ({exit_code}): {stderr}")]
    NonZeroExit {
        /// Exit code reported by the child process.
        exit_code: i32,
        /// Captured stdout (may be empty).
        stdout: String,
        /// Captured stderr (may be empty).
        stderr: String,
    },
    /// Spawning the child process or reading its output failed.
    #[error("IO error running CLI: {0}")]
    Io(String),
}

/// Testable wrapper around "run a subprocess" semantics.
///
/// Implementations must be `Send + Sync` because they are held inside
/// state owned by multiple Tauri command invocations concurrently.
/// Arguments are passed individually (`&[&str]`) — never as a single
/// shell string — so no implementation ever needs to shell-escape.
pub trait CommandRunner: Send + Sync {
    /// Execute `cmd` with the given argv and working directory, waiting
    /// for it to finish and returning the captured stdout + stderr +
    /// exit code. A non-zero exit **is** a [`CliError::NonZeroExit`]
    /// rather than a successful `CliOutput` — that keeps dispatcher
    /// code from having to inspect exit codes manually.
    fn run(&self, cmd: &str, args: &[&str], cwd: &Path) -> Result<CliOutput, CliError>;
}

/// Real implementation of [`CommandRunner`] that shells out via
/// [`std::process::Command`].
///
/// On Windows the command is launched with `CREATE_NO_WINDOW` so the
/// user never sees a flashing console window. On other platforms this
/// is a no-op.
#[derive(Debug, Clone, Default)]
pub struct SystemRunner;

impl SystemRunner {
    /// Construct a new [`SystemRunner`].
    pub fn new() -> Self {
        Self
    }
}

impl CommandRunner for SystemRunner {
    fn run(&self, cmd: &str, args: &[&str], cwd: &Path) -> Result<CliOutput, CliError> {
        let mut command = std::process::Command::new(cmd);
        command.args(args).current_dir(cwd);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let output = command.output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                CliError::NotFound(cmd.to_string())
            } else {
                CliError::Io(e.to_string())
            }
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        if output.status.success() {
            Ok(CliOutput {
                stdout,
                stderr,
                exit_code,
            })
        } else {
            Err(CliError::NonZeroExit {
                exit_code,
                stdout,
                stderr,
            })
        }
    }
}

/// One recorded invocation of [`MockRunner::run`].
///
/// Tests compare the exact argv to assert that every CLI flag is
/// emitted per-argument and that no shell escaping is ever attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCall {
    /// The command name that was executed (e.g. `"gh"`).
    pub cmd: String,
    /// Arguments passed individually to the process.
    pub args: Vec<String>,
    /// Working directory the command was launched in.
    pub cwd: PathBuf,
}

/// Canned response keyed on a `cmd` plus an argv prefix.
///
/// The prefix lets a single mock entry match multiple distinct
/// invocations that share a common argv head — for example both
/// `["repo", "edit", "--description", "x"]` and
/// `["repo", "edit", "--add-topic", "y"]` are matched by the prefix
/// `["repo", "edit"]`.
#[derive(Clone)]
struct MockResponse {
    cmd: String,
    args_prefix: Vec<String>,
    result: Result<CliOutput, CliErrorSnapshot>,
}

/// Serializable snapshot of a [`CliError`] used inside [`MockRunner`].
///
/// [`CliError`] carries a `std::io::Error` which is not `Clone`. The
/// mock needs to hand out canned failures from multiple tests, so we
/// keep a small `Clone`-friendly snapshot and rehydrate on demand.
#[derive(Debug, Clone)]
enum CliErrorSnapshot {
    NotFound(String),
    NonZeroExit {
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
    Io(String),
}

impl From<CliErrorSnapshot> for CliError {
    fn from(s: CliErrorSnapshot) -> Self {
        match s {
            CliErrorSnapshot::NotFound(m) => CliError::NotFound(m),
            CliErrorSnapshot::NonZeroExit {
                exit_code,
                stdout,
                stderr,
            } => CliError::NonZeroExit {
                exit_code,
                stdout,
                stderr,
            },
            CliErrorSnapshot::Io(m) => CliError::Io(m),
        }
    }
}

/// Test-only [`CommandRunner`] that records every call and returns
/// canned responses.
///
/// Construction pattern:
///
/// ```ignore
/// use app_core::commands::command_runner::{MockRunner, CliOutput, CommandRunner};
/// let runner = MockRunner::new();
/// runner.expect("gh", &["repo", "view"], Ok(CliOutput { stdout: "{}".into(), stderr: "".into(), exit_code: 0 }));
/// let out = runner.run("gh", &["repo", "view", "--json", "description"], std::path::Path::new(".")).unwrap();
/// assert_eq!(out.stdout, "{}");
/// assert_eq!(runner.calls()[0].args, vec!["repo", "view", "--json", "description"]);
/// ```
#[derive(Default, Clone)]
pub struct MockRunner {
    calls: Arc<Mutex<Vec<RecordedCall>>>,
    responses: Arc<Mutex<Vec<MockResponse>>>,
    /// Fallback when no response matches — lets quick tests just assert
    /// on argv and ignore the return value.
    default_ok_stdout: Arc<Mutex<Option<String>>>,
}

impl MockRunner {
    /// Create an empty mock runner with no pre-programmed responses.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a canned response for any call whose argv starts with
    /// `args_prefix`. Later registrations win over earlier ones so
    /// tests can re-program the mock mid-scenario.
    pub fn expect(&self, cmd: &str, args_prefix: &[&str], result: Result<CliOutput, CliError>) {
        let snapshot = match result {
            Ok(out) => Ok(out),
            Err(CliError::NotFound(m)) => Err(CliErrorSnapshot::NotFound(m)),
            Err(CliError::NonZeroExit {
                exit_code,
                stdout,
                stderr,
            }) => Err(CliErrorSnapshot::NonZeroExit {
                exit_code,
                stdout,
                stderr,
            }),
            Err(CliError::Io(m)) => Err(CliErrorSnapshot::Io(m)),
        };
        let mut r = self.responses.lock().expect("mock responses poisoned");
        r.push(MockResponse {
            cmd: cmd.to_string(),
            args_prefix: args_prefix.iter().map(|s| s.to_string()).collect(),
            result: snapshot,
        });
    }

    /// Register a canned successful stdout response returned when no
    /// other response matches.
    pub fn set_default_ok(&self, stdout: impl Into<String>) {
        let mut slot = self
            .default_ok_stdout
            .lock()
            .expect("mock default poisoned");
        *slot = Some(stdout.into());
    }

    /// Snapshot of every call this runner received.
    pub fn calls(&self) -> Vec<RecordedCall> {
        self.calls
            .lock()
            .expect("mock calls poisoned")
            .iter()
            .cloned()
            .collect()
    }

    /// True if a call matching `(cmd, args)` exactly was recorded.
    pub fn was_called_with(&self, cmd: &str, args: &[&str]) -> bool {
        self.calls().iter().any(|c| {
            c.cmd == cmd
                && c.args.len() == args.len()
                && c.args.iter().zip(args).all(|(a, b)| a == b)
        })
    }
}

impl CommandRunner for MockRunner {
    fn run(&self, cmd: &str, args: &[&str], cwd: &Path) -> Result<CliOutput, CliError> {
        let call = RecordedCall {
            cmd: cmd.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: cwd.to_path_buf(),
        };
        self.calls.lock().expect("mock calls poisoned").push(call);

        // Walk responses in reverse so the most recent registration wins.
        let responses = self.responses.lock().expect("mock responses poisoned");
        for r in responses.iter().rev() {
            if r.cmd != cmd {
                continue;
            }
            if r.args_prefix.len() > args.len() {
                continue;
            }
            let matches = r
                .args_prefix
                .iter()
                .zip(args.iter())
                .all(|(expected, actual)| expected.as_str() == *actual);
            if matches {
                return r.result.clone().map_err(CliError::from);
            }
        }
        drop(responses);

        if let Some(out) = self
            .default_ok_stdout
            .lock()
            .expect("mock default poisoned")
            .clone()
        {
            return Ok(CliOutput {
                stdout: out,
                stderr: String::new(),
                exit_code: 0,
            });
        }

        Err(CliError::NonZeroExit {
            exit_code: 1,
            stdout: String::new(),
            stderr: format!(
                "MockRunner: no canned response for ({cmd}, {args:?}); use MockRunner::expect()",
            ),
        })
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_runner_captures_exact_argv() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Ok(CliOutput {
                stdout: "hello".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let out = runner
            .run(
                "gh",
                &["repo", "view", "--json", "description"],
                Path::new("/tmp"),
            )
            .unwrap();
        assert_eq!(out.stdout, "hello");
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].cmd, "gh");
        assert_eq!(calls[0].args, vec!["repo", "view", "--json", "description"]);
        assert_eq!(calls[0].cwd, PathBuf::from("/tmp"));
    }

    #[test]
    fn mock_runner_was_called_with_exact_match() {
        let runner = MockRunner::new();
        runner.set_default_ok("");
        runner
            .run("gh", &["label", "create", "bug"], Path::new("."))
            .unwrap();
        assert!(runner.was_called_with("gh", &["label", "create", "bug"]));
        assert!(!runner.was_called_with("gh", &["label", "delete", "bug"]));
    }

    #[test]
    fn mock_runner_returns_canned_error() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &["repo", "view"],
            Err(CliError::NonZeroExit {
                exit_code: 2,
                stdout: String::new(),
                stderr: "not authenticated".into(),
            }),
        );
        let err = runner
            .run("gh", &["repo", "view"], Path::new("."))
            .unwrap_err();
        match err {
            CliError::NonZeroExit {
                exit_code, stderr, ..
            } => {
                assert_eq!(exit_code, 2);
                assert_eq!(stderr, "not authenticated");
            }
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }

    #[test]
    fn mock_runner_no_response_yields_error_by_default() {
        let runner = MockRunner::new();
        let err = runner
            .run("gh", &["repo", "view"], Path::new("."))
            .unwrap_err();
        assert!(matches!(err, CliError::NonZeroExit { .. }));
    }

    #[test]
    fn mock_runner_later_expect_wins() {
        let runner = MockRunner::new();
        runner.expect(
            "gh",
            &[],
            Ok(CliOutput {
                stdout: "first".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        runner.expect(
            "gh",
            &[],
            Ok(CliOutput {
                stdout: "second".into(),
                stderr: String::new(),
                exit_code: 0,
            }),
        );
        let out = runner.run("gh", &["whatever"], Path::new(".")).unwrap();
        assert_eq!(out.stdout, "second");
    }

    #[test]
    fn mock_runner_default_ok_when_no_match() {
        let runner = MockRunner::new();
        runner.set_default_ok("fallback");
        let out = runner.run("gh", &["anything"], Path::new(".")).unwrap();
        assert_eq!(out.stdout, "fallback");
    }

    #[cfg(not(windows))]
    #[test]
    fn system_runner_echo_returns_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let runner = SystemRunner::new();
        let out = runner.run("echo", &["hello"], dir.path()).unwrap();
        assert_eq!(out.stdout.trim(), "hello");
        assert_eq!(out.exit_code, 0);
    }

    #[cfg(not(windows))]
    #[test]
    fn system_runner_not_found_returns_not_found_error() {
        let dir = tempfile::tempdir().unwrap();
        let runner = SystemRunner::new();
        let err = runner
            .run("some-binary-that-does-not-exist-xyz", &[], dir.path())
            .unwrap_err();
        assert!(matches!(err, CliError::NotFound(_)));
    }

    #[cfg(not(windows))]
    #[test]
    fn system_runner_nonzero_exit_returns_structured_error() {
        let dir = tempfile::tempdir().unwrap();
        let runner = SystemRunner::new();
        let err = runner.run("false", &[], dir.path()).unwrap_err();
        match err {
            CliError::NonZeroExit { exit_code, .. } => assert_ne!(exit_code, 0),
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }
}
