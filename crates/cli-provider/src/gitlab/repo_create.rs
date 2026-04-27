//! `glab repo create` adapter: maps `CreateRepoInput` to the glab CLI flags
//! and parses the resulting project URL into a [`RepoCreated`].

use forge_provider::{CreateRepoInput, ForgeError, RepoCreated};

use super::GitLabCli;

impl GitLabCli {
    pub(super) fn create_repo_impl(
        &self,
        input: CreateRepoInput,
    ) -> Result<RepoCreated, ForgeError> {
        // `glab` exposes visibility as boolean flags (`--private`,
        // `--public`, `--internal`) rather than `--visibility <vis>`. We
        // mirror the gh adapter and just pick `--private` / `--public`.
        let visibility = if input.private {
            "--private"
        } else {
            "--public"
        };
        // glab repo create <name> --private|--public --defaultBranch main
        // Output examples:
        //   "Project created. https://gitlab.com/me/foo"
        //   "https://gitlab.com/me/foo"
        let stdout = self
            .run(&[
                "repo",
                "create",
                &input.name,
                visibility,
                "--defaultBranch",
                "main",
            ])
            .map_err(map_create_error)?;
        let web_url = parse_repo_url(&stdout)
            .ok_or_else(|| ForgeError::Cli("glab produced no URL".into()))?;
        let clone_url = format!("{web_url}.git");
        Ok(RepoCreated { clone_url, web_url })
    }
}

/// Pick the first http(s):// token from `glab repo create` stdout.
///
/// `glab` typically prints `"Project created. https://gitlab.com/me/foo"` —
/// the URL is wedged into a sentence on the same line. We tokenise on
/// whitespace and pick the first URL-shaped token, trimming trailing
/// punctuation a stray `.` or `,` might leave on it (and a `/` in case
/// glab ever appends a path slash).
///
/// (The GitHub sibling scans `lines()` instead because `gh` puts the URL
/// alone on its own line. Don't unify the two — token-scanning is more
/// permissive and would loosen the gh side unnecessarily.)
fn parse_repo_url(stdout: &str) -> Option<String> {
    stdout
        .split_whitespace()
        .find(|w| w.starts_with("https://") || w.starts_with("http://"))
        .map(|s| s.trim_end_matches(['.', ',', '/']).to_string())
}

/// Translate `glab`'s name-collision wording into [`ForgeError::NameTaken`].
///
/// GitLab's REST surface returns "has already been taken" / "Path has
/// already been taken"; older / different forms may also include "already
/// exists". Both are mapped here.
fn map_create_error(err: ForgeError) -> ForgeError {
    let ForgeError::Cli(msg) = &err else {
        return err;
    };
    let lower = msg.to_lowercase();
    if lower.contains("has already been taken") || lower.contains("already exists") {
        return ForgeError::NameTaken;
    }
    err
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_repo_url_picks_first_https() {
        let out = "Project created. https://gitlab.com/me/foo. See https://docs.gitlab.com for next steps.\n";
        assert_eq!(
            parse_repo_url(out).as_deref(),
            Some("https://gitlab.com/me/foo")
        );
    }

    #[test]
    fn parse_repo_url_handles_url_only() {
        assert_eq!(
            parse_repo_url("https://gitlab.com/me/foo\n").as_deref(),
            Some("https://gitlab.com/me/foo")
        );
    }

    #[test]
    fn parse_repo_url_returns_none_when_missing() {
        assert!(parse_repo_url("created.\n").is_none());
        assert!(parse_repo_url("").is_none());
    }

    #[test]
    fn parse_repo_url_strips_trailing_punctuation() {
        assert_eq!(
            parse_repo_url("https://gitlab.com/me/foo.").as_deref(),
            Some("https://gitlab.com/me/foo")
        );
    }

    #[test]
    fn parse_repo_url_strips_trailing_slash() {
        assert_eq!(
            parse_repo_url("https://gitlab.com/me/foo/").as_deref(),
            Some("https://gitlab.com/me/foo")
        );
    }

    #[test]
    fn map_create_error_recognises_taken() {
        let err = ForgeError::Cli("Path has already been taken".into());
        assert!(matches!(map_create_error(err), ForgeError::NameTaken));
    }

    #[test]
    fn map_create_error_recognises_already_exists() {
        let err = ForgeError::Cli("project already exists".into());
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
