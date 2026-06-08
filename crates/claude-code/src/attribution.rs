//! Commit attribution patterns for Claude Code.
//!
//! Detects AI-authored commits by checking for:
//! - `Authored-by:` footer (Claude Code convention)
//! - `Co-authored-by:` trailer mentioning Claude/Anthropic
//! - Author name containing "Claude"

use ai_provider::{AttributionMatch, AttributionPattern};

/// Returns the attribution patterns for Claude Code.
pub fn patterns() -> Vec<AttributionPattern> {
    vec![
        AttributionPattern {
            kind: AttributionMatch::Footer,
            // Require Claude/Anthropic on the line — a bare `Authored-by:` is
            // also the project's HUMAN commit convention.
            pattern: "Authored-by:.*(?i)(claude|anthropic)".to_string(),
        },
        AttributionPattern {
            kind: AttributionMatch::Trailer,
            pattern: "Co-authored-by:.*(?i)(claude|anthropic)".to_string(),
        },
        AttributionPattern {
            kind: AttributionMatch::AuthorName,
            pattern: "(?i)claude".to_string(),
        },
    ]
}

/// Check if a commit was authored by Claude Code.
pub fn is_ai_authored(message: &str, author: &str) -> bool {
    for line in message.lines() {
        let trimmed = line.trim();
        // Both the `Authored-by:` footer and the `Co-authored-by:` trailer only
        // count as AI-authored when Claude/Anthropic appears on the SAME line.
        // A bare `Authored-by:` is also the project's human commit convention
        // (root CLAUDE.md), so matching it unconditionally mislabels human
        // commits as AI-authored.
        if trimmed.starts_with("Authored-by:") || trimmed.starts_with("Co-authored-by:") {
            let lower = trimmed.to_lowercase();
            if lower.contains("claude") || lower.contains("anthropic") {
                return true;
            }
        }
    }

    if author.to_lowercase().contains("claude") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_authored_by_footer() {
        assert!(is_ai_authored(
            "feat: add feature\n\nAuthored-by: Claude Code",
            "Adolfo"
        ));
    }

    #[test]
    fn detects_co_authored_trailer() {
        assert!(is_ai_authored(
            "fix: bug\n\nCo-authored-by: Claude <claude@anthropic.com>",
            "Adolfo"
        ));
    }

    #[test]
    fn detects_anthropic_trailer() {
        assert!(is_ai_authored(
            "chore: cleanup\n\nCo-authored-by: Bot <bot@anthropic.com>",
            "Adolfo"
        ));
    }

    #[test]
    fn detects_claude_author() {
        assert!(is_ai_authored("fix: something", "Claude"));
    }

    #[test]
    fn human_commit_not_detected() {
        assert!(!is_ai_authored(
            "feat: add feature\n\nSigned-off-by: Adolfo",
            "Adolfo Fuentes"
        ));
    }

    #[test]
    fn human_authored_by_trailer_not_detected() {
        // The project's own human convention uses `Authored-by:` — it must NOT
        // be flagged as AI-authored unless Claude/Anthropic is on the line.
        assert!(!is_ai_authored(
            "feat: add feature\n\nAuthored-by: Adolfo Fuentes <a@example.com>",
            "Adolfo Fuentes"
        ));
    }

    #[test]
    fn patterns_has_three_entries() {
        assert_eq!(patterns().len(), 3);
    }

    #[test]
    fn case_insensitive_author() {
        assert!(is_ai_authored("msg", "CLAUDE BOT"));
        assert!(is_ai_authored("msg", "claude-code"));
    }
}
