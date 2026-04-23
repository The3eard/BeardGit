//! Shared Codex rollout-file metadata helpers.
//!
//! Codex persists each exec / interactive run as a JSONL file under
//! `~/.codex/sessions/YYYY/MM/DD/rollout-<timestamp>-<uuid>.jsonl`. The first
//! line of every file is a `session_meta` event carrying session-level
//! metadata; subsequent lines are timeline events (turn contexts, response
//! items, tool calls, etc.). We only need the first line.
//!
//! **Probe verified on 2026-04-20** against `codex-cli 0.121.0`. A real
//! invocation (`codex exec "print hello"` in `/tmp/codex-test`) produced the
//! following first-line shape (trimmed):
//!
//! ```json
//! {
//!   "timestamp": "2026-04-20T21:58:55.163Z",
//!   "type": "session_meta",
//!   "payload": {
//!     "id": "019dace7-5260-7762-a330-072ff38df69f",
//!     "timestamp": "2026-04-20T21:58:54.320Z",
//!     "cwd": "/private/tmp/codex-test",
//!     "originator": "codex_exec",
//!     "cli_version": "0.121.0",
//!     "source": "exec",
//!     "model_provider": "openai"
//!   }
//! }
//! ```
//!
//! This module used to host a full session-listing path with PID-less
//! liveness heuristics; after the transcript-first rewrite the only
//! consumers are [`super::conversations`] (which walks the same tree)
//! and a couple of shared deserialisation / timestamp-parsing helpers.

use std::time::Duration;

use serde::Deserialize;

/// Rollouts older than this are skipped at the directory-walk level.
///
/// Codex writes one JSONL file per conversation under
/// `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` with no built-in
/// pruning — a year of daily use accumulates thousands of historical
/// rollouts that almost never match the current project anyway.
/// Reading + parsing the first line of every file put the whole AI
/// Sessions view behind a ~600 ms filesystem walk on every click.
///
/// Anything touched within the last 30 days is still considered
/// "recent enough to surface" — that window covers typical "show me
/// last week's runs" UX without re-parsing the entire history.
pub const DISCOVERY_WINDOW: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// First-line JSON shape — only the fields we read.
#[derive(Debug, Deserialize)]
pub(crate) struct SessionMeta {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) payload: SessionMetaPayload,
}

/// Inner `payload` object of a Codex `session_meta` record.
#[derive(Debug, Deserialize)]
pub(crate) struct SessionMetaPayload {
    pub(crate) id: String,
    /// ISO 8601 timestamp — e.g. `"2026-04-20T21:58:54.320Z"`.
    pub(crate) timestamp: Option<String>,
    pub(crate) cwd: Option<String>,
}

/// Very small RFC-3339 parser — avoids pulling `chrono` just to convert the
/// timestamp to Unix milliseconds. Returns `None` on any parsing failure.
pub(crate) fn parse_rfc3339_to_unix_millis(ts: &str) -> Option<u64> {
    // Expected: `YYYY-MM-DDTHH:MM:SS[.fff]Z`
    let (date, rest) = ts.split_once('T')?;
    let time = rest.strip_suffix('Z').unwrap_or(rest);
    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;

    let (hms, frac) = match time.split_once('.') {
        Some((hms, frac)) => (hms, frac),
        None => (time, "0"),
    };
    let mut hms_parts = hms.split(':');
    let hour: u32 = hms_parts.next()?.parse().ok()?;
    let minute: u32 = hms_parts.next()?.parse().ok()?;
    let second: u32 = hms_parts.next()?.parse().ok()?;

    // Civil → Julian day number (Howard Hinnant's algorithm).
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days_since_epoch = era * 146_097 + doe as i64 - 719_468;

    let secs = days_since_epoch * 86_400
        + i64::from(hour) * 3_600
        + i64::from(minute) * 60
        + i64::from(second);
    if secs < 0 {
        return None;
    }
    let millis_frac: u64 = {
        // Use up to three fractional digits.
        let mut buf = String::from(frac);
        buf.truncate(3);
        while buf.len() < 3 {
            buf.push('0');
        }
        buf.parse().ok()?
    };
    Some(secs as u64 * 1_000 + millis_frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_parses_typical_value() {
        let millis = parse_rfc3339_to_unix_millis("2026-04-20T21:58:54.320Z").unwrap();
        // 2026-04-20T21:58:54.320Z = 1776722334320ms
        assert_eq!(millis, 1_776_722_334_320);
    }

    #[test]
    fn rfc3339_rejects_garbage() {
        assert!(parse_rfc3339_to_unix_millis("nope").is_none());
    }
}
