//! CI log preprocessing — strips provider-specific noise and normalizes format.
//!
//! Each CI provider (GitLab, GitHub) wraps job log output with timestamps,
//! stream codes, section markers, and annotation directives that clutter the
//! reading experience. This module strips that noise and produces clean,
//! line-numbered output while preserving ANSI color/style codes for rendering.
//!
//! # Supported transformations
//!
//! ## GitLab
//! - Strips ISO-8601 timestamps and stream codes (`00O`, `01E`, `00O+`).
//! - Extracts `HH:MM:SS` time prefix from the timestamp.
//! - Removes `section_start:` / `section_end:` marker lines.
//! - Strips `\x1b[0K` (erase-line) sequences that GitLab injects.
//!
//! ## GitHub
//! - Strips `##[group]` / `##[endgroup]` directives.
//! - Converts `##[error]` and `##[warning]` annotations to ANSI color codes.
//! - Extracts `HH:MM:SS` from ISO-8601 timestamps when present.

use crate::ProviderKind;

/// Preprocess a raw CI log into clean, line-numbered output.
///
/// Each non-empty content line is prefixed with a left-aligned line number
/// (5 chars wide) followed by the cleaned text. Section markers, empty lines,
/// and other noise are dropped entirely and do not consume line numbers.
pub fn preprocess_ci_log(raw: &str, kind: ProviderKind) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    let mut output = Vec::new();
    let mut line_num = 1u32;

    for line in &lines {
        let cleaned = match kind {
            ProviderKind::GitLab => preprocess_gitlab_line(line),
            ProviderKind::GitHub => preprocess_github_line(line),
        };
        if let Some(text) = cleaned {
            output.push(format!("{:<5}{}", line_num, text));
            line_num += 1;
        }
    }

    output.join("\n")
}

/// Clean a single GitLab CI log line.
///
/// Expected format: `ISO_TIMESTAMP STREAM_CODE CONTENT`
/// e.g. `2026-04-01T11:08:41.057014Z 00O \x1b[32;1mJob succeeded\x1b[0;m`
///
/// Returns `None` for section markers and empty lines.
fn preprocess_gitlab_line(line: &str) -> Option<String> {
    let line = line.trim_end();
    if line.is_empty() {
        return None;
    }

    // Strip \x1b[0K (erase-line) sequences injected by GitLab runners.
    let cleaned = line.replace("\x1b[0K", "");

    // Check for section markers at the line level (before timestamp parsing).
    let check = cleaned.trim_start();
    if check.starts_with("section_start:") || check.starts_with("section_end:") {
        return None;
    }

    // Try to parse: ISO_TIMESTAMP STREAM_CODE CONTENT
    // Timestamp is 27 chars: 2026-04-01T11:08:41.057014Z
    if cleaned.len() >= 31 {
        let ts = &cleaned[..27];
        if ts.len() >= 19
            && ts.as_bytes().get(4) == Some(&b'-')
            && ts.as_bytes().get(10) == Some(&b'T')
        {
            let time = &ts[11..19]; // HH:MM:SS
            if time.as_bytes().get(2) == Some(&b':') && time.as_bytes().get(5) == Some(&b':') {
                // Skip space + stream code (00O, 01E, 00O+, etc.)
                let rest = &cleaned[28..];
                // Stream code is 3–4 alphanumeric chars possibly followed by '+'
                let content_start = rest
                    .find(|c: char| !c.is_ascii_alphanumeric() && c != '+')
                    .unwrap_or(rest.len())
                    .max(3);
                let content = if content_start < rest.len() {
                    rest[content_start..].trim_start()
                } else {
                    ""
                };
                if content.is_empty() {
                    return None;
                }
                // Check for section markers in extracted content too.
                if content.starts_with("section_start:") || content.starts_with("section_end:") {
                    return None;
                }
                return Some(format!("{} {}", time, content));
            }
        }
    }

    // No timestamp — pass through as-is (e.g. bare ANSI output).
    Some(cleaned)
}

/// Clean a single GitHub Actions log line.
///
/// Strips `##[group]`/`##[endgroup]` directives and converts `##[error]`/
/// `##[warning]` annotations to ANSI color codes. Extracts `HH:MM:SS` from
/// ISO-8601 timestamps when present.
///
/// Returns `None` for group directives and empty lines.
fn preprocess_github_line(line: &str) -> Option<String> {
    let line = line.trim_end();
    if line.is_empty() {
        return None;
    }

    // Strip ##[group] / ##[endgroup]
    if line.starts_with("##[group]") || line == "##[endgroup]" {
        return None;
    }

    // Convert ##[error] / ##[warning] to ANSI color codes.
    if let Some(msg) = line.strip_prefix("##[error]") {
        return Some(format!("\x1b[31;1m{}\x1b[0m", msg));
    }
    if let Some(msg) = line.strip_prefix("##[warning]") {
        return Some(format!("\x1b[33;1m{}\x1b[0m", msg));
    }

    // Try to strip ISO timestamp prefix.
    if line.len() > 28
        && line.as_bytes().get(4) == Some(&b'-')
        && line.as_bytes().get(10) == Some(&b'T')
    {
        let time = &line[11..19];
        if time.as_bytes().get(2) == Some(&b':') {
            let rest = line[28..].trim_start();
            if rest.is_empty() {
                return None;
            }
            return Some(format!("{} {}", time, rest));
        }
    }

    Some(line.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitlab_basic_line() {
        let line = "2026-04-01T11:08:41.057014Z 00O \x1b[32;1mJob succeeded\x1b[0;m";
        let result = preprocess_gitlab_line(line).unwrap();
        assert!(result.starts_with("11:08:41"));
        assert!(result.contains("\x1b[32;1m")); // ANSI preserved
        assert!(result.contains("Job succeeded"));
    }

    #[test]
    fn test_gitlab_section_stripped() {
        let line = "2026-04-01T11:08:01.723628Z 00O section_end:1775041681:step_script";
        assert!(preprocess_gitlab_line(line).is_none());
    }

    #[test]
    fn test_gitlab_section_start_with_ansi() {
        let line = "2026-04-01T11:03:43.423362Z 00O section_start:1775041423:prepare_executor";
        assert!(preprocess_gitlab_line(line).is_none());
    }

    #[test]
    fn test_gitlab_erase_line_stripped() {
        let line = "2026-04-01T11:03:43.423220Z 00O \x1b[0KRunning with gitlab-runner\x1b[0;m";
        let result = preprocess_gitlab_line(line).unwrap();
        assert!(!result.contains("\x1b[0K"));
        assert!(result.contains("Running with gitlab-runner"));
    }

    #[test]
    fn test_gitlab_empty_content_skipped() {
        let line = "2026-04-01T11:08:41.057014Z 00O ";
        assert!(preprocess_gitlab_line(line).is_none());
    }

    #[test]
    fn test_gitlab_passthrough_no_timestamp() {
        let line = "some output without timestamp";
        let result = preprocess_gitlab_line(line).unwrap();
        assert_eq!(result, "some output without timestamp");
    }

    #[test]
    fn test_github_error_annotation() {
        let line = "##[error]Process exited with code 1";
        let result = preprocess_github_line(line).unwrap();
        assert!(result.contains("\x1b[31;1m"));
        assert!(result.contains("Process exited with code 1"));
    }

    #[test]
    fn test_github_warning_annotation() {
        let line = "##[warning]Node.js 16 is deprecated";
        let result = preprocess_github_line(line).unwrap();
        assert!(result.contains("\x1b[33;1m"));
        assert!(result.contains("Node.js 16 is deprecated"));
    }

    #[test]
    fn test_github_group_stripped() {
        assert!(preprocess_github_line("##[group]Run npm test").is_none());
        assert!(preprocess_github_line("##[endgroup]").is_none());
    }

    #[test]
    fn test_github_timestamp_stripped() {
        let line = "2026-04-01T11:08:41.0000000Z npm test passed";
        let result = preprocess_github_line(line).unwrap();
        assert!(result.starts_with("11:08:41"));
        assert!(result.contains("npm test passed"));
    }

    #[test]
    fn test_github_passthrough() {
        let line = "plain output line";
        let result = preprocess_github_line(line).unwrap();
        assert_eq!(result, "plain output line");
    }

    #[test]
    fn test_line_numbers() {
        let raw = "2026-04-01T11:08:41.057014Z 00O line one\n\
                    2026-04-01T11:08:41.057014Z 00O section_start:123:test\n\
                    2026-04-01T11:08:41.057014Z 00O line two";
        let result = preprocess_ci_log(raw, ProviderKind::GitLab);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("1    "));
        assert!(lines[1].starts_with("2    "));
    }

    #[test]
    fn test_empty_input() {
        let result = preprocess_ci_log("", ProviderKind::GitLab);
        assert!(result.is_empty());
    }

    #[test]
    fn test_github_line_numbers() {
        let raw = "##[group]Setup\n\
                    2026-04-01T11:08:41.0000000Z step one\n\
                    ##[endgroup]\n\
                    2026-04-01T11:08:42.0000000Z step two";
        let result = preprocess_ci_log(raw, ProviderKind::GitHub);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("1    "));
        assert!(lines[1].starts_with("2    "));
    }
}
