//! Command builders for headless and interactive execution.

use std::path::Path;
use std::process::Command;

use ai_provider::{AiBackgroundRunInput, AiError, AiProvider, ExecuteOptions};

/// Build a headless execution command using `opencode -p <prompt>`.
pub fn build_execute_command(
    provider: &dyn AiProvider,
    prompt: &str,
    cwd: &Path,
    options: &ExecuteOptions,
) -> Result<Command, AiError> {
    let binary = provider
        .detect_binary()
        .ok_or_else(|| AiError::BinaryNotFound(provider.binary_name().into()))?;

    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("-p").arg(prompt);

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }

    for arg in &options.extra_args {
        cmd.arg(arg);
    }

    Ok(cmd)
}

/// Build an interactive terminal launch command.
pub fn build_interactive_cmd(provider: &dyn AiProvider, cwd: &Path) -> Result<Command, AiError> {
    let binary = provider
        .detect_binary()
        .ok_or_else(|| AiError::BinaryNotFound(provider.binary_name().into()))?;

    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    Ok(cmd)
}

/// Build a **headless background** OpenCode command.
///
/// OpenCode accepts the prompt via `-p <text>` in its non-interactive mode
/// (`opencode run`). As of this writing there's no first-class `--skill` flag
/// or `--prompt-file` — skill/saved-prompt content is inlined into
/// `input.prompt` at the coordinator layer.
pub fn build_background_command(binary: &Path, input: &AiBackgroundRunInput) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(&input.worktree_path);
    cmd.arg("run");
    cmd.arg("-p").arg(&input.prompt);
    cmd
}

/// Build a headless execute command from a known binary path (for testing).
pub fn build_execute_command_from_binary(
    binary: &Path,
    prompt: &str,
    cwd: &Path,
    options: &ExecuteOptions,
) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("-p").arg(prompt);

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }

    for arg in &options.extra_args {
        cmd.arg(arg);
    }

    cmd
}

/// Build an interactive command from a known binary path (for testing).
pub fn build_interactive_cmd_from_binary(binary: &Path, cwd: &Path) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn debug_cmd(cmd: &Command) -> String {
        format!("{cmd:?}")
    }

    #[test]
    fn execute_command_has_p_flag() {
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/opencode"),
            "hello",
            Path::new("/tmp/repo"),
            &ExecuteOptions::default(),
        );
        let debug = debug_cmd(&cmd);
        assert!(debug.contains("\"-p\""));
        assert!(debug.contains("hello"));
    }

    #[test]
    fn execute_command_with_model() {
        let opts = ExecuteOptions {
            model: Some("gpt-4o".into()),
            ..Default::default()
        };
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/opencode"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        );
        let debug = debug_cmd(&cmd);
        assert!(debug.contains("--model"));
        assert!(debug.contains("gpt-4o"));
    }

    #[test]
    fn execute_command_with_extra_args() {
        let opts = ExecuteOptions {
            extra_args: vec!["--verbose".into()],
            ..Default::default()
        };
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/opencode"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        );
        let debug = debug_cmd(&cmd);
        assert!(debug.contains("--verbose"));
    }

    #[test]
    fn background_command_has_run_p_and_prompt() {
        let input = AiBackgroundRunInput {
            provider: ai_provider::AiProviderKind::OpenCode,
            worktree_path: PathBuf::from("/tmp/wt/run-1"),
            prompt: "bump version".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            auto_accept_permissions: false,
        };
        let cmd = build_background_command(&PathBuf::from("/usr/bin/opencode"), &input);
        let d = debug_cmd(&cmd);
        assert!(d.contains("run"));
        assert!(d.contains("\"-p\""));
        assert!(d.contains("bump version"));
    }

    #[test]
    fn interactive_command_no_prompt_flag() {
        let cmd = build_interactive_cmd_from_binary(
            &PathBuf::from("/usr/bin/opencode"),
            Path::new("/tmp/repo"),
        );
        let debug = debug_cmd(&cmd);
        assert!(debug.contains("/usr/bin/opencode"));
        assert!(!debug.contains("\"-p\""));
    }
}
