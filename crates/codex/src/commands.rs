//! Command builders for headless and interactive Codex execution.
//!
//! These functions build [`std::process::Command`] objects but never execute
//! them — execution is the caller's responsibility (`app-core` via TaskManager
//! or TerminalManager).

use std::path::Path;
use std::process::Command;

use ai_provider::{AiBackgroundRunInput, AiError, ExecuteOptions};

/// Build a headless execution command: `codex exec -p <prompt>`.
///
/// Adds `--model` if set in `options.model`. Appends any `options.extra_args`.
pub fn build_execute_command(
    binary: &Path,
    prompt: &str,
    cwd: &Path,
    options: &ExecuteOptions,
) -> Result<Command, AiError> {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("exec").arg("-p").arg(prompt);

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }

    for arg in &options.extra_args {
        cmd.arg(arg);
    }

    Ok(cmd)
}

/// Build an interactive terminal launch command: just `codex` with `cwd` set.
pub fn build_interactive_cmd(binary: &Path, cwd: &Path) -> Result<Command, AiError> {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    Ok(cmd)
}

/// Build a resume command for an existing session id.
///
/// Shape: `codex exec resume <session_id> -C <cwd>`. Stdout is configured
/// for piping so callers can stream output (matches the Claude Code idiom).
/// `session_id` is passed verbatim — Codex UUIDs are filename-safe.
pub fn build_resume_session_cmd(binary: &Path, session_id: &str, cwd: &Path) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("exec")
        .arg("resume")
        .arg(session_id)
        .arg("-C")
        .arg(cwd);
    cmd.stdout(std::process::Stdio::piped());
    cmd
}

/// Build a **headless background** Codex command.
///
/// Codex doesn't have first-class `--skill` or `--prompt-file` flags at the
/// time of writing — the plan calls for inlining saved-prompt / skill content
/// into the free-text prompt at the coordinator layer, then passing the
/// combined string via `-p`. `input.prompt` here is expected to already be
/// the combined string.
///
/// `cwd` is set to `input.worktree_path`. We pass `--skip-git-repo-check` so
/// Codex doesn't refuse to run inside a linked worktree the very first time.
pub fn build_background_command(binary: &Path, input: &AiBackgroundRunInput) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(&input.worktree_path);
    cmd.arg("exec");
    if input.auto_accept_permissions {
        cmd.arg("--dangerously-bypass-approvals-and-sandbox");
    }
    cmd.arg("--skip-git-repo-check");
    cmd.arg("-p").arg(&input.prompt);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn debug(cmd: &Command) -> String {
        format!("{cmd:?}")
    }

    #[test]
    fn execute_command_has_exec_and_prompt_flag() {
        let cmd = build_execute_command(
            &PathBuf::from("/usr/bin/codex"),
            "hello",
            Path::new("/tmp/repo"),
            &ExecuteOptions::default(),
        )
        .unwrap();
        let d = debug(&cmd);
        assert!(d.contains("exec"));
        assert!(d.contains("-p"));
        assert!(d.contains("hello"));
    }

    #[test]
    fn execute_command_with_model() {
        let opts = ExecuteOptions {
            model: Some("gpt-4o".into()),
            ..Default::default()
        };
        let cmd = build_execute_command(
            &PathBuf::from("/usr/bin/codex"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        )
        .unwrap();
        let d = debug(&cmd);
        assert!(d.contains("--model"));
        assert!(d.contains("gpt-4o"));
    }

    #[test]
    fn execute_command_with_extra_args() {
        let opts = ExecuteOptions {
            extra_args: vec!["--no-ansi".into()],
            ..Default::default()
        };
        let cmd = build_execute_command(
            &PathBuf::from("/usr/bin/codex"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        )
        .unwrap();
        let d = debug(&cmd);
        assert!(d.contains("--no-ansi"));
    }

    #[test]
    fn background_command_has_exec_p_and_skip_check() {
        let input = AiBackgroundRunInput {
            provider: ai_provider::AiProviderKind::Codex,
            worktree_path: PathBuf::from("/tmp/wt/run-1"),
            prompt: "refactor cli args".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            auto_accept_permissions: false,
        };
        let cmd = build_background_command(&PathBuf::from("/usr/bin/codex"), &input);
        let d = debug(&cmd);
        assert!(d.contains("exec"));
        assert!(d.contains("--skip-git-repo-check"));
        assert!(d.contains("-p"));
        assert!(d.contains("refactor cli args"));
        // Auto-accept flag defaults OFF.
        assert!(!d.contains("--dangerously-bypass-approvals-and-sandbox"));
    }

    #[test]
    fn background_command_auto_accept_adds_bypass_flag() {
        let input = AiBackgroundRunInput {
            provider: ai_provider::AiProviderKind::Codex,
            worktree_path: PathBuf::from("/tmp/wt/run-2"),
            prompt: "x".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            auto_accept_permissions: true,
        };
        let cmd = build_background_command(&PathBuf::from("/usr/bin/codex"), &input);
        let d = debug(&cmd);
        assert!(d.contains("--dangerously-bypass-approvals-and-sandbox"));
    }

    #[test]
    fn interactive_command_sets_cwd_no_extra_args() {
        let cmd = build_interactive_cmd(&PathBuf::from("/usr/bin/codex"), Path::new("/tmp/repo"))
            .unwrap();
        let d = debug(&cmd);
        assert!(d.contains("/usr/bin/codex"));
        assert!(!d.contains("exec"));
        assert!(!d.contains("-p"));
    }

    #[test]
    fn resume_command_contains_exec_resume_and_session_id() {
        let cmd = build_resume_session_cmd(
            &PathBuf::from("/usr/bin/codex"),
            "019dace7-5260-7762",
            Path::new("/tmp/repo"),
        );
        let args: Vec<&std::ffi::OsStr> = cmd.get_args().collect();
        assert!(args.iter().any(|a| a.to_str() == Some("exec")));
        assert!(args.iter().any(|a| a.to_str() == Some("resume")));
        assert!(
            args.iter()
                .any(|a| a.to_str() == Some("019dace7-5260-7762"))
        );
        assert!(args.iter().any(|a| a.to_str() == Some("-C")));
        assert!(args.iter().any(|a| a.to_str() == Some("/tmp/repo")));
    }

    #[test]
    fn resume_command_has_correct_argv_order() {
        let cmd = build_resume_session_cmd(
            &PathBuf::from("/usr/bin/codex"),
            "sess-id",
            Path::new("/tmp/repo"),
        );
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        // `exec` must precede `resume`; `resume` must precede the session id.
        let exec_idx = args.iter().position(|a| a == "exec").unwrap();
        let resume_idx = args.iter().position(|a| a == "resume").unwrap();
        let id_idx = args.iter().position(|a| a == "sess-id").unwrap();
        assert!(exec_idx < resume_idx);
        assert!(resume_idx < id_idx);
    }
}
