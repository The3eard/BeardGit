//! Command builders for headless and interactive execution.

use std::path::Path;
use std::process::Command;

use ai_provider::{AiBackgroundRunInput, AiError, AiProvider, ExecuteOptions, OutputFormat};

/// Build a headless execution command with `--print`.
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
    cmd.arg("--print").arg(prompt);

    match options.output_format {
        OutputFormat::Text => {
            cmd.arg("--output-format").arg("text");
        }
        OutputFormat::Json => {
            cmd.arg("--output-format").arg("json");
        }
    }

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }
    if let Some(budget) = options.max_budget {
        cmd.arg("--max-budget-usd").arg(budget.to_string());
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

/// Build a worktree launch command with `--worktree`.
pub fn build_worktree_cmd(
    provider: &dyn AiProvider,
    cwd: &Path,
    name: Option<&str>,
) -> Option<Command> {
    let binary = provider.detect_binary()?;
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("--worktree");
    if let Some(n) = name {
        cmd.arg(n);
    }
    Some(cmd)
}

/// Build a headless **background** command used by the AI background
/// coordinator.
///
/// Flags applied:
/// - `--print` — non-TTY, single-shot mode
/// - `--output-format json-stream` — line-delimited events for streaming
/// - `--dangerously-skip-permissions` — only when
///   `input.auto_accept_permissions` is `true`. Default off; users opt in.
/// - `--skill <name>` — when a skill is selected
/// - `--resume <id>` — when a session id is provided
///
/// The prompt itself is NOT added as an argument — it is piped on stdin by
/// the coordinator. If `saved_prompt_path` is set, callers read its contents
/// and concatenate before feeding stdin.
pub fn build_background_command(
    provider: &dyn AiProvider,
    input: &AiBackgroundRunInput,
) -> Result<Command, AiError> {
    let binary = provider
        .detect_binary()
        .ok_or_else(|| AiError::BinaryNotFound(provider.binary_name().into()))?;
    Ok(build_background_command_from_binary(&binary, input))
}

/// Same as [`build_background_command`] but without running `detect_binary`.
/// Used by unit tests and by callers that already know the executable path.
pub fn build_background_command_from_binary(
    binary: &Path,
    input: &AiBackgroundRunInput,
) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(&input.worktree_path);
    cmd.arg("--print");
    cmd.arg("--output-format").arg("stream-json");
    // Verbose is required alongside stream-json in non-interactive mode so
    // Claude emits incremental events instead of buffering the whole response.
    cmd.arg("--verbose");

    if input.auto_accept_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }
    if let Some(skill) = input.skill.as_deref() {
        cmd.arg("--skill").arg(skill);
    }
    if let Some(session_id) = input.resume_session_id.as_deref() {
        cmd.arg("--resume").arg(session_id);
    }
    cmd
}

/// Build a command from a known binary path (for testing without detection).
pub fn build_execute_command_from_binary(
    binary: &Path,
    prompt: &str,
    cwd: &Path,
    options: &ExecuteOptions,
) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("--print").arg(prompt);

    match options.output_format {
        OutputFormat::Text => {
            cmd.arg("--output-format").arg("text");
        }
        OutputFormat::Json => {
            cmd.arg("--output-format").arg("json");
        }
    }

    if let Some(ref model) = options.model {
        cmd.arg("--model").arg(model);
    }
    if let Some(budget) = options.max_budget {
        cmd.arg("--max-budget-usd").arg(budget.to_string());
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

/// Build a worktree command from a known binary path (for testing).
pub fn build_worktree_cmd_from_binary(binary: &Path, cwd: &Path, name: Option<&str>) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("--worktree");
    if let Some(n) = name {
        cmd.arg(n);
    }
    cmd
}

/// Build a resume session command from a known binary path (for testing).
pub fn build_resume_cmd_from_binary(binary: &Path, cwd: &Path, session_id: &str) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd);
    cmd.arg("--resume").arg(session_id);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper to get args from a Command (uses Debug format).
    fn get_command_debug(cmd: &Command) -> String {
        format!("{cmd:?}")
    }

    #[test]
    fn execute_command_has_print_flag() {
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            "hello",
            Path::new("/tmp/repo"),
            &ExecuteOptions::default(),
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--print"));
        assert!(debug.contains("hello"));
    }

    #[test]
    fn execute_command_json_format() {
        let opts = ExecuteOptions {
            output_format: OutputFormat::Json,
            ..Default::default()
        };
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--output-format"));
        assert!(debug.contains("json"));
    }

    #[test]
    fn execute_command_with_model() {
        let opts = ExecuteOptions {
            model: Some("opus".into()),
            ..Default::default()
        };
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--model"));
        assert!(debug.contains("opus"));
    }

    #[test]
    fn execute_command_with_budget() {
        let opts = ExecuteOptions {
            max_budget: Some(0.50),
            ..Default::default()
        };
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--max-budget-usd"));
        assert!(debug.contains("0.5"));
    }

    #[test]
    fn execute_command_with_extra_args() {
        let opts = ExecuteOptions {
            extra_args: vec!["--verbose".into(), "--no-cache".into()],
            ..Default::default()
        };
        let cmd = build_execute_command_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            "test",
            Path::new("/tmp/repo"),
            &opts,
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--verbose"));
        assert!(debug.contains("--no-cache"));
    }

    #[test]
    fn interactive_command_sets_cwd() {
        let cmd = build_interactive_cmd_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            Path::new("/tmp/repo"),
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("/usr/bin/claude"));
        assert!(!debug.contains("--print"));
    }

    #[test]
    fn worktree_command_has_flag() {
        let cmd = build_worktree_cmd_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            Path::new("/tmp/repo"),
            None,
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--worktree"));
    }

    #[test]
    fn worktree_command_with_name() {
        let cmd = build_worktree_cmd_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            Path::new("/tmp/repo"),
            Some("my-feature"),
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--worktree"));
        assert!(debug.contains("my-feature"));
    }

    #[test]
    fn background_command_has_print_and_streamjson() {
        let input = AiBackgroundRunInput {
            provider: ai_provider::AiProviderKind::ClaudeCode,
            worktree_path: PathBuf::from("/tmp/wt/run-1"),
            prompt: "anything".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            auto_accept_permissions: false,
        };
        let cmd = build_background_command_from_binary(&PathBuf::from("/usr/bin/claude"), &input);
        let d = get_command_debug(&cmd);
        assert!(d.contains("--print"));
        assert!(d.contains("--output-format"));
        assert!(d.contains("stream-json"));
        assert!(d.contains("--verbose"));
        // Prompt is piped via stdin — must NOT appear in argv.
        assert!(!d.contains("anything"));
        // Auto-accept defaults OFF.
        assert!(!d.contains("--dangerously-skip-permissions"));
    }

    #[test]
    fn background_command_auto_accept_adds_skip_perm_flag() {
        let input = AiBackgroundRunInput {
            provider: ai_provider::AiProviderKind::ClaudeCode,
            worktree_path: PathBuf::from("/tmp/wt/run-2"),
            prompt: "".into(),
            skill: None,
            saved_prompt_path: None,
            resume_session_id: None,
            auto_accept_permissions: true,
        };
        let cmd = build_background_command_from_binary(&PathBuf::from("/usr/bin/claude"), &input);
        let d = get_command_debug(&cmd);
        assert!(d.contains("--dangerously-skip-permissions"));
    }

    #[test]
    fn background_command_with_skill_and_resume() {
        let input = AiBackgroundRunInput {
            provider: ai_provider::AiProviderKind::ClaudeCode,
            worktree_path: PathBuf::from("/tmp/wt/run-3"),
            prompt: "".into(),
            skill: Some("code-review".into()),
            saved_prompt_path: None,
            resume_session_id: Some("sess-42".into()),
            auto_accept_permissions: false,
        };
        let cmd = build_background_command_from_binary(&PathBuf::from("/usr/bin/claude"), &input);
        let d = get_command_debug(&cmd);
        assert!(d.contains("--skill"));
        assert!(d.contains("code-review"));
        assert!(d.contains("--resume"));
        assert!(d.contains("sess-42"));
    }

    #[test]
    fn resume_command_has_session_id() {
        let cmd = build_resume_cmd_from_binary(
            &PathBuf::from("/usr/bin/claude"),
            Path::new("/tmp/repo"),
            "sess-abc123",
        );
        let debug = get_command_debug(&cmd);
        assert!(debug.contains("--resume"));
        assert!(debug.contains("sess-abc123"));
    }
}
