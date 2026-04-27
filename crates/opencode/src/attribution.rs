//! Heuristics for detecting commits produced by the OpenCode CLI.
//!
//! OpenCode records its identity via `Co-authored-by:` trailers or by
//! placing `opencode` in the commit author. Matching is case-insensitive
//! and conservative — we prefer to under-report than to mislabel a
//! human-authored commit. Mirrors the Codex detector.

/// Return `true` when `message` / `author` look like they came from OpenCode.
pub fn is_ai_authored(message: &str, author: &str) -> bool {
    for line in message.lines() {
        let lower = line.trim().to_lowercase();
        if lower.starts_with("co-authored-by:") && lower.contains("opencode") {
            return true;
        }
    }

    author.to_lowercase().contains("opencode")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailer_with_opencode_matches() {
        let msg = "feat: thing\n\nCo-authored-by: opencode <bot@opencode.ai>\n";
        assert!(is_ai_authored(msg, "Alice <alice@example.com>"));
    }

    #[test]
    fn author_containing_opencode_matches() {
        assert!(is_ai_authored(
            "feat: thing",
            "OpenCode Bot <opencode@example.com>"
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
        let msg = "fix: bug\n\nCO-AUTHORED-BY: OPENCODE <ops@opencode.ai>";
        assert!(is_ai_authored(msg, "Alice"));
    }

    #[test]
    fn case_insensitive_author() {
        assert!(is_ai_authored("fix: x", "OPENCODE <bot@opencode.ai>"));
    }

    #[test]
    fn codex_trailer_does_not_match_opencode() {
        // Guard against cross-provider false positives — a Codex trailer
        // must not pull "opencode" out of thin air.
        let msg = "feat: x\n\nCo-authored-by: Codex CLI <codex@openai.com>";
        assert!(!is_ai_authored(msg, "Alice"));
    }
}
