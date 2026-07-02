//! Heuristics for detecting commits produced by the OpenCode CLI.
//!
//! Thin wrapper over [`ai_provider_common::is_ai_authored`], parameterized by
//! [`crate::SPEC`] (needle: `opencode`). Matching is case-insensitive and
//! conservative — we prefer to under-report than to mislabel a human commit.

use crate::SPEC;

/// Return `true` when `message` / `author` look like they came from OpenCode.
pub fn is_ai_authored(message: &str, author: &str) -> bool {
    ai_provider_common::is_ai_authored(&SPEC, message, author)
}
