//! Foreground process detection for terminal sessions.
//!
//! Uses platform-specific APIs to determine which process is currently
//! in the foreground of a PTY session. On macOS/Linux, uses
//! `libc::tcgetpgrp()` to get the foreground process group, then reads
//! the process name. On Windows, returns `None` (not supported yet).

/// Get the name of the foreground process for the given PTY master file descriptor.
///
/// Returns `None` if detection is not supported on this platform, or if the
/// process name could not be determined (e.g. the shell itself is in the
/// foreground, the fd is invalid, or the child process has exited).
#[cfg(unix)]
pub fn get_foreground_process_name(master_fd: i32) -> Option<String> {
    if master_fd < 0 {
        return None;
    }

    // Get the foreground process group ID from the PTY master.
    // SAFETY: tcgetpgrp takes a raw fd and returns a pid_t; no memory is
    // accessed and failure returns -1 with errno set. We check for <= 0.
    let pgrp = unsafe { libc::tcgetpgrp(master_fd) };
    if pgrp <= 0 {
        return None;
    }

    read_process_name(pgrp as u32)
}

/// Windows stub — foreground process detection not currently supported.
#[cfg(windows)]
pub fn get_foreground_process_name(_master_fd: i32) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
fn read_process_name(pid: u32) -> Option<String> {
    let comm_path = format!("/proc/{}/comm", pid);
    std::fs::read_to_string(comm_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
fn read_process_name(pid: u32) -> Option<String> {
    // Use `ps -o comm= -p <pid>` to get the process name on macOS.
    // `comm=` tells ps to output just the command, no header.
    let output = std::process::Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        return None;
    }

    // ps may return the full path on macOS — extract just the binary name.
    name.rsplit('/')
        .next()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

// Other unix platforms (BSDs, etc.) — fall back to Linux-style /proc if
// available, otherwise return None.
#[cfg(all(unix, not(target_os = "linux"), not(target_os = "macos")))]
fn read_process_name(pid: u32) -> Option<String> {
    let comm_path = format!("/proc/{}/comm", pid);
    std::fs::read_to_string(comm_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_for_invalid_fd() {
        // fd -1 should not panic, just return None
        let result = get_foreground_process_name(-1);
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_for_closed_fd() {
        // fd 999 is almost certainly not open
        let result = get_foreground_process_name(999);
        assert!(result.is_none());
    }
}
