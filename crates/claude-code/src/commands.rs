//! Command builders for headless and interactive execution.

use std::path::Path;
use std::process::Command;

use ai_provider::{AiError, AiProvider, ExecuteOptions, OutputFormat};

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
