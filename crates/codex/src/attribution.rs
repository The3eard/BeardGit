//! Heuristics for detecting commits produced by the Codex CLI.
//!
//! Codex historically records its identity via `Co-authored-by:` trailers
//! (OpenAI's `codex-cli` or similar) or by placing `codex` in the commit
//! author. Matching is case-insensitive and conservative — we prefer to
//! under-report than to mislabel a human-authored commit.

/// Return `true` when `message` / `author` look like they came from Codex.
pub fn is_ai_authored(message: &str, author: &str) -> bool {
    for line in message.lines() {
        let lower = line.trim().to_lowercase();
        if lower.starts_with("co-authored-by:") && lower.contains("codex") {
            return true;
        }
    }

    author.to_lowercase().contains("codex")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailer_with_codex_matches() {
        let msg = "feat: thing\n\nCo-authored-by: Codex CLI <codex@openai.com>\n";
        assert!(is_ai_authored(msg, "Alice <alice@example.com>"));
    }

    #[test]
    fn author_containing_codex_matches() {
        assert!(is_ai_authored(
            "feat: thing",
            "OpenAI Codex <codex@openai.com>"
        ));
    }

    #[test]
    fn unrelated_trailer_does_not_match() {
        let msg = "feat: thing\n\nCo-authored-by: Dependabot <bot@example.com>\n";
        assert!(!is_ai_authored(msg, "Alice <alice@example.com>"));
    }

    #[test]
    fn plain_message_does_not_match() {
        assert!(!is_ai_authored("docs: tweak", "Alice <alice@example.com>"));
    }

    #[test]
    fn case_insensitive_trailer() {
        let msg = "fix: bug\n\nCO-AUTHORED-BY: CODEX <codex@openai.com>";
        assert!(is_ai_authored(msg, "Alice"));
    }
}
