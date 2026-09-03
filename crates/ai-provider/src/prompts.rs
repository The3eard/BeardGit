//! Prompt text for the specialized headless actions.
//!
//! Kept separate from the command builders so the caller can decide how the
//! prompt reaches the CLI (argv or stdin) while the wording lives in one place.
//! The text is provider-neutral: the same prompt goes to Claude Code, Codex
//! and OpenCode.

/// Prompt for generating a commit message from a staged diff.
///
/// The reply is inserted verbatim into the commit-message box, which is why
/// the prompt says so instead of shouting "ONLY".
pub fn commit_message(diff: &str) -> String {
    format!(
        "Write a concise git commit message for this diff in conventional \
         commits format (type(scope): description). Your whole reply is \
         inserted verbatim as the commit message, so return only the message \
         itself.\n\n{diff}"
    )
}

/// Prompt for reviewing a working-tree or staged diff.
///
/// The reply is saved as a markdown file next to the repo, hence the shape
/// guidance.
pub fn review(diff: &str) -> String {
    format!(
        "Review this code diff. Report bugs, security issues, performance \
         problems, and style concerns. Be concise. The reply is saved as a \
         markdown file: group findings by severity, cite file and line, and \
         say so plainly if there is nothing worth reporting.\n\n{diff}"
    )
}

/// Prompt for answering a free-form question about a piece of code.
pub fn analysis(content: &str, question: &str) -> String {
    format!("{question}\n\n{content}")
}

/// Prompt for generating a pull/merge request description.
pub fn pr_description(diff: &str) -> String {
    format!(
        "Generate a pull request description for this diff. Include a summary \
         section and a list of key changes. Use markdown formatting.\n\n{diff}"
    )
}

/// Prompt for reviewing a pull/merge request diff.
pub fn pr_review(diff: &str) -> String {
    format!(
        "Review this pull request diff. Report bugs, security issues, design \
         concerns, and suggest improvements. The reply is saved as a markdown \
         file: group findings by severity, cite file and line, and say so \
         plainly if there is nothing worth reporting.\n\n{diff}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_end_with_the_payload() {
        assert!(commit_message("DIFF").ends_with("\n\nDIFF"));
        assert!(review("DIFF").ends_with("\n\nDIFF"));
        assert!(pr_description("DIFF").ends_with("\n\nDIFF"));
        assert!(pr_review("DIFF").ends_with("\n\nDIFF"));
        assert_eq!(analysis("CODE", "Q?"), "Q?\n\nCODE");
    }

    #[test]
    fn no_pressure_language() {
        for p in [
            commit_message(""),
            review(""),
            pr_description(""),
            pr_review(""),
        ] {
            for word in ["ONLY", "MUST", "NEVER", "ALWAYS", "Be thorough"] {
                assert!(!p.contains(word), "{word:?} found in {p:?}");
            }
        }
    }
}
