//! Shell detection for the current platform.
//!
//! Reads `$SHELL` on macOS/Linux. On Windows, reads `ComSpec` env var
//! with fallback to `powershell.exe`.

use crate::types::TerminalError;

/// Detect the system default shell.
///
/// Returns the shell path as a string (e.g. `/bin/zsh`, `powershell.exe`).
pub fn detect_shell() -> Result<String, TerminalError> {
    detect_shell_inner()
}

#[cfg(unix)]
fn detect_shell_inner() -> Result<String, TerminalError> {
    std::env::var("SHELL").map_err(|_| TerminalError::NoShellDetected)
}

#[cfg(windows)]
fn detect_shell_inner() -> Result<String, TerminalError> {
    std::env::var("ComSpec")
        .or_else(|_| {
            // Try powershell as fallback
            let ps = "powershell.exe";
            if which_shell(ps) {
                Ok(ps.to_string())
            } else {
                Err(std::env::VarError::NotPresent)
            }
        })
        .map_err(|_| TerminalError::NoShellDetected)
}

#[cfg(windows)]
fn which_shell(name: &str) -> bool {
    std::process::Command::new("where")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_shell_returns_something() {
        let shell = detect_shell().expect("should detect a shell");
        assert!(!shell.is_empty(), "shell path should not be empty");
    }

    #[cfg(unix)]
    #[test]
    fn detect_shell_respects_env() {
        let original = std::env::var("SHELL").ok();
        unsafe { std::env::set_var("SHELL", "/usr/bin/fish") };
        let result = detect_shell().unwrap();
        assert_eq!(result, "/usr/bin/fish");
        if let Some(orig) = original {
            unsafe { std::env::set_var("SHELL", orig) };
        }
    }
}
