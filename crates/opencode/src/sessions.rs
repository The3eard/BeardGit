//! Shared OpenCode CLI runner used by transcript-first listing code.
//!
//! OpenCode stores its sessions in a SQLite DB at
//! `~/.local/share/opencode/opencode.db`. Rather than depending on SQLite
//! and risking schema drift, we shell out to the first-class
//! `opencode session list --format json` command and parse the JSON array.
//!
//! All CLI shell-outs go through the [`SessionRunner`] trait so unit tests
//! can replay canned JSON without actually spawning `opencode`.
//!
//! This module used to also host a full PID-less session lister with a
//! recency-based liveness heuristic; after the transcript-first rewrite
//! the only remaining consumer is [`crate::conversations`], which needs
//! the shelled-out JSON to build an [`ai_provider::AiConversation`] list.

use std::path::PathBuf;
use std::process::Command;

/// Abstract runner for the `opencode` CLI.
///
/// Implemented by [`CliSessionRunner`] in production and by test doubles in
/// unit tests. The trait only needs to shell out to `opencode <args>` and
/// return combined stdout.
pub trait SessionRunner {
    /// Run `opencode` with the given args; return stdout as a `String`.
    fn run(&self, args: &[&str]) -> std::io::Result<String>;
}

/// Production runner that spawns `opencode` on PATH with `--log-level ERROR`
/// to silence the default INFO chatter that otherwise ends up on stderr.
pub struct CliSessionRunner {
    binary: PathBuf,
}

impl CliSessionRunner {
    /// Create a runner bound to the given `opencode` binary path.
    pub fn new(binary: PathBuf) -> Self {
        Self { binary }
    }
}

impl SessionRunner for CliSessionRunner {
    fn run(&self, args: &[&str]) -> std::io::Result<String> {
        let output = Command::new(&self.binary)
            .args(args)
            .arg("--log-level")
            .arg("ERROR")
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
