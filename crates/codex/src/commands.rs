//! Command builders for headless and interactive Codex execution.
//!
//! These functions build [`std::process::Command`] objects but never execute
//! them — execution is the caller's responsibility (`app-core` via TaskManager
//! or TerminalManager).

use std::path::Path;
use std::process::Command;

use ai_provider::{AiError, ExecuteOptions};

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
    fn interactive_command_sets_cwd_no_extra_args() {
        let cmd = build_interactive_cmd(&PathBuf::from("/usr/bin/codex"), Path::new("/tmp/repo"))
            .unwrap();
        let d = debug(&cmd);
        assert!(d.contains("/usr/bin/codex"));
        assert!(!d.contains("exec"));
        assert!(!d.contains("-p"));
    }
}
