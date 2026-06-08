//! Stderr-pattern classification for OpenCode CLI invocations.
//!
//! OpenCode returns unstructured stderr on failure. We scan for a small
//! set of known substrings to distinguish auth / rate-limit failures
//! from everything else, so the UI can surface actionable guidance.
//! Unknown errors fall through to [`AiError::Other`] with the raw
//! stderr attached.
//!
//! Patterns are case-insensitive and conservative — if OpenCode changes
//! its wording, the classifier quietly reports `Other(...)` instead of
//! misclassifying. Mirrors the Codex classifier module.

use ai_provider::AiError;

/// Try to map a stderr blob onto a specific [`AiError`] variant.
///
/// Returns `None` when no pattern matches; the caller should wrap the raw
/// stderr in [`AiError::Other`] so the UI can still show the message.
pub fn classify_stderr(stderr: &str) -> Option<AiError> {
    let lower = stderr.to_lowercase();

    if lower.contains("command not found") || lower.contains("no such file or directory") {
        return Some(AiError::BinaryNotFound("opencode".into()));
    }

    if lower.contains("not authenticated")
        || lower.contains("please run `opencode providers`")
        || lower.contains("please run opencode providers")
        || lower.contains("authentication required")
        || lower.contains("unauthorized")
        || mentions_http_status(&lower, "401")
    {
        return Some(AiError::AuthExpired(first_line_or_stderr(stderr)));
    }

    if lower.contains("rate limit")
        || lower.contains("too many requests")
        || mentions_http_status(&lower, "429")
        || lower.contains("quota")
    {
        return Some(AiError::RateLimited(first_line_or_stderr(stderr)));
    }

    None
}

/// Match an HTTP status `code` only when it appears in a status-ish context.
/// A bare `401`/`429` shows up routinely in byte counts, ports, durations,
/// and timestamps, so matching the raw substring misclassifies unrelated
/// failures as auth/rate-limit errors.
fn mentions_http_status(lower: &str, code: &str) -> bool {
    [
        format!("http {code}"),
        format!("http/1.1 {code}"),
        format!("http/2 {code}"),
        format!("status {code}"),
        format!("status: {code}"),
        format!("status code {code}"),
        format!("code {code}"),
        format!("({code})"),
        format!("[{code}]"),
    ]
    .iter()
    .any(|p| lower.contains(p.as_str()))
}

/// Classify the stderr and fall back to `Other` on unknown output.
pub fn classify_or_other(stderr: &str) -> AiError {
    classify_stderr(stderr).unwrap_or_else(|| AiError::Other(stderr.trim().to_string()))
}

fn first_line_or_stderr(stderr: &str) -> String {
    stderr
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(stderr)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_not_found_pattern() {
        let err = classify_stderr("opencode: command not found").unwrap();
        assert!(matches!(err, AiError::BinaryNotFound(_)));
    }

    #[test]
    fn binary_not_found_on_no_such_file() {
        let err = classify_stderr("/bin/sh: opencode: No such file or directory").unwrap();
        assert!(matches!(err, AiError::BinaryNotFound(_)));
    }

    #[test]
    fn auth_expired_patterns() {
        for sample in [
            "Error: not authenticated. Please run `opencode providers`.",
            "please run opencode providers to reconnect",
            "Authentication required",
            "Unauthorized: token expired",
            "HTTP 401",
        ] {
            let err = classify_stderr(sample).unwrap_or_else(|| panic!("no match for {sample:?}"));
            assert!(matches!(err, AiError::AuthExpired(_)), "sample: {sample}");
        }
    }

    #[test]
    fn rate_limited_patterns() {
        for sample in [
            "rate limit exceeded",
            "Too Many Requests",
            "HTTP 429 Too Many Requests",
            "quota exceeded for this account",
        ] {
            let err = classify_stderr(sample).unwrap_or_else(|| panic!("no match for {sample:?}"));
            assert!(matches!(err, AiError::RateLimited(_)), "sample: {sample}");
        }
    }

    #[test]
    fn unknown_output_returns_none() {
        assert!(classify_stderr("something unexpected happened").is_none());
    }

    #[test]
    fn classify_or_other_falls_back() {
        let err = classify_or_other("brand new error message");
        assert!(matches!(err, AiError::Other(ref s) if s.contains("brand new")));
    }

    #[test]
    fn classify_or_other_uses_classifier_when_match() {
        let err = classify_or_other("HTTP 429 retry later");
        assert!(matches!(err, AiError::RateLimited(_)));
    }

    #[test]
    fn first_line_helper_picks_first_non_empty() {
        // "Unauthorized" stays as-is; embedded later lines are ignored.
        let err = classify_stderr("\n\nUnauthorized: expired\nstacktrace here").unwrap();
        match err {
            AiError::AuthExpired(msg) => assert_eq!(msg, "Unauthorized: expired"),
            _ => panic!("expected AuthExpired"),
        }
    }
}
