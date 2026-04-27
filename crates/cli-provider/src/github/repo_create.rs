//! `gh repo create` adapter: maps `CreateRepoInput` to the gh CLI flags
//! and parses the resulting repo URL into a [`RepoCreated`].

use forge_provider::{CreateRepoInput, ForgeError, RepoCreated};

use super::GitHubCli;

impl GitHubCli {
    pub(super) fn create_repo_impl(
        &self,
        input: CreateRepoInput,
    ) -> Result<RepoCreated, ForgeError> {
        let visibility = if input.private {
            "--private"
        } else {
            "--public"
        };
        // gh repo create <name> --private|--public
        // Stdout typically prints a single web URL line; informational
        // lines (Created repository …, deprecation notes) go to stderr,
        // but we parse defensively in case that ever changes.
        let stdout = self
            .run(&["repo", "create", &input.name, visibility])
            .map_err(map_create_error)?;
        let web_url =
            parse_repo_url(&stdout).ok_or_else(|| ForgeError::Cli("gh produced no URL".into()))?;
        let clone_url = format!("{web_url}.git");
        Ok(RepoCreated { clone_url, web_url })
    }
}

/// Pick the first http(s):// line from `gh repo create` stdout.
///
/// `gh` writes the canonical web URL on its own line on stdout; informational
/// notices (e.g. deprecation banners, "Created repository …") go to stderr
/// and so don't reach us. We scan line-by-line so a future stdout notice
/// would not poison the parsed URL.
///
/// (The GitLab sibling uses `split_whitespace` instead, because `glab` packs
/// the URL into a sentence like `"Project created. https://…"`. Don't unify
/// the two — line-scanning is stricter and is the right choice for `gh`.)
fn parse_repo_url(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("https://") || l.starts_with("http://"))
        .map(|l| l.trim_end_matches(['.', ',']).to_string())
}

/// Translate `gh`'s name-collision wording into [`ForgeError::NameTaken`].
///
/// `gh` surfaces several variants (REST `"already exists"`, GraphQL
/// `"already been taken"`); both are mapped here. Anything else passes
/// through unchanged.
fn map_create_error(err: ForgeError) -> ForgeError {
    let ForgeError::Cli(msg) = &err else {
        return err;
    };
    let lower = msg.to_lowercase();
    if lower.contains("already exists") || lower.contains("already been taken") {
        return ForgeError::NameTaken;
    }
    err
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_repo_url_from_single_line() {
        assert_eq!(
            parse_repo_url("https://github.com/me/foo\n").as_deref(),
            Some("https://github.com/me/foo")
        );
    }

    #[test]
    fn parse_repo_url_skips_leading_notice() {
        let out = "Note: gh repo create now defaults to private.\nhttps://github.com/me/foo\n";
        assert_eq!(
            parse_repo_url(out).as_deref(),
            Some("https://github.com/me/foo")
        );
    }

    #[test]
    fn parse_repo_url_returns_none_for_garbage() {
        assert!(parse_repo_url("created.\n").is_none());
        assert!(parse_repo_url("").is_none());
    }

    #[test]
    fn parse_repo_url_strips_trailing_punctuation() {
        assert_eq!(
            parse_repo_url("https://github.com/me/foo.\n").as_deref(),
            Some("https://github.com/me/foo")
        );
    }

    #[test]
    fn map_create_error_recognises_already_exists() {
        let err = ForgeError::Cli("Error: repository already exists".into());
        assert!(matches!(map_create_error(err), ForgeError::NameTaken));
    }

    #[test]
    fn map_create_error_recognises_already_been_taken() {
        let err = ForgeError::Cli("GraphQL: Name has already been taken".into());
        assert!(matches!(map_create_error(err), ForgeError::NameTaken));
    }

    #[test]
    fn map_create_error_passes_through_unknown() {
        let err = ForgeError::Cli("network failure".into());
        assert!(matches!(map_create_error(err), ForgeError::Cli(_)));
    }

    #[test]
    fn map_create_error_passes_through_non_cli() {
        let err = ForgeError::NotAuthenticated("nope".into());
        assert!(matches!(
            map_create_error(err),
            ForgeError::NotAuthenticated(_)
        ));
    }
}
