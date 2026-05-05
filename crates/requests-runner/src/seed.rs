//! Seeds `.beardgit/requests/` with the Quickstart starter pack of
//! `.http` files.
//!
//! [`seed_quickstart_pack`] is the **default** onboarding pack. Uses
//! public, no-auth REST APIs (`JSONPlaceholder`, `httpbin.org`) so the
//! user can hit Send immediately and see a real response. This is what
//! new users land on; no forge token, no provider configured, nothing
//! to set up.

use std::path::Path;

use crate::error::RequestsError;

/// JSON body of an empty `_env/default.json` — the same stub the
/// "Start empty" SeedPrompt path writes inline. Used to detect whether
/// the existing default env on disk is still the untouched stub (and
/// can be safely overwritten with a richer default) or has user edits
/// (and must be preserved verbatim).
const EMPTY_DEFAULT_ENV: &str =
    "{\n  \"$schema\": \"beardgit-env/v1\",\n  \"vars\": {},\n  \"secrets\": []\n}\n";

/// `_env/default.json` contents seeded alongside the Quickstart pack.
/// Wires up the `base_url`, `httpbin_base_url`, and `post_id` vars
/// referenced by the seeded `.http` files so the user can hit Send
/// without first opening the env editor.
const QUICKSTART_DEFAULT_ENV: &str = concat!(
    "{\n",
    "  \"$schema\": \"beardgit-env/v1\",\n",
    "  \"vars\": {\n",
    "    \"base_url\": \"https://jsonplaceholder.typicode.com\",\n",
    "    \"httpbin_base_url\": \"https://httpbin.org\",\n",
    "    \"post_id\": \"1\"\n",
    "  },\n",
    "  \"secrets\": []\n",
    "}\n",
);

/// Seed the **Quickstart** starter pack under
/// `.beardgit/requests/quickstart/`.
///
/// Drops nine `.http` files split across two sub-folders:
///
/// - `quickstart/jsonplaceholder/` — five CRUD samples against the
///   public [JSONPlaceholder] fake REST API (no auth required, supports
///   list / get / create / update / delete on `/posts`).
/// - `quickstart/httpbin/` — four request-inspection samples against
///   [httpbin.org] (echo, form POST, custom status code, slow response
///   for testing the Cancel button).
///
/// Also seeds (or upgrades) `_env/default.json` with the matching
/// variables (`base_url`, `httpbin_base_url`, `post_id`) so every file
/// resolves out of the box. **Existing customised `default.json` files
/// are preserved** — the env is only written when missing or when its
/// content matches the empty-stub written by the "Start empty" path.
///
/// Returns the relative paths under `.beardgit/requests/` that were
/// written, in insertion order. The first entry is
/// `quickstart/jsonplaceholder/list-posts.http` — the most natural file
/// to land on after seeding.
///
/// [JSONPlaceholder]: https://jsonplaceholder.typicode.com
/// [httpbin.org]: https://httpbin.org
pub fn seed_quickstart_pack(project_root: &Path) -> Result<Vec<String>, RequestsError> {
    let req_root = project_root.join(".beardgit").join("requests");
    let quickstart_dir = req_root.join("quickstart");
    let jsonph_dir = quickstart_dir.join("jsonplaceholder");
    let httpbin_dir = quickstart_dir.join("httpbin");
    std::fs::create_dir_all(&jsonph_dir)?;
    std::fs::create_dir_all(&httpbin_dir)?;

    // _env/default.json: only write it when missing or still the
    // untouched empty stub. This protects user-edited envs.
    let env_dir = req_root.join("_env");
    std::fs::create_dir_all(&env_dir)?;
    let env_path = env_dir.join("default.json");
    let should_write_env = match std::fs::read_to_string(&env_path) {
        Ok(existing) => existing == EMPTY_DEFAULT_ENV,
        Err(_) => true, // missing or unreadable → safe to write fresh
    };
    if should_write_env {
        std::fs::write(&env_path, QUICKSTART_DEFAULT_ENV)?;
    }

    let mut written = vec![];
    for (rel, body) in quickstart_files() {
        let target = req_root.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, body)?;
        written.push((*rel).to_string());
    }
    Ok(written)
}

/// `.http` payloads for the Quickstart pack, in the order surfaced to
/// the UI (the first entry is what gets auto-selected after seeding).
fn quickstart_files() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "quickstart/jsonplaceholder/list-posts.http",
            "# @name List posts\nGET {{base_url}}/posts\nAccept: application/json\n",
        ),
        (
            "quickstart/jsonplaceholder/get-post.http",
            "# @name Get a post\nGET {{base_url}}/posts/{{post_id}}\nAccept: application/json\n",
        ),
        (
            "quickstart/jsonplaceholder/create-post.http",
            "# @name Create a post\nPOST {{base_url}}/posts\nContent-Type: application/json\nAccept: application/json\n\n{\n  \"title\": \"hello from BeardGit\",\n  \"body\": \"this is a sample post\",\n  \"userId\": 1\n}\n",
        ),
        (
            "quickstart/jsonplaceholder/update-post.http",
            "# @name Update a post\nPUT {{base_url}}/posts/{{post_id}}\nContent-Type: application/json\nAccept: application/json\n\n{\n  \"id\": 1,\n  \"title\": \"updated title\",\n  \"body\": \"updated body\",\n  \"userId\": 1\n}\n",
        ),
        (
            "quickstart/jsonplaceholder/delete-post.http",
            "# @name Delete a post\nDELETE {{base_url}}/posts/{{post_id}}\n",
        ),
        (
            "quickstart/httpbin/inspect-request.http",
            "# @name Inspect request (echoes back what we sent)\nGET {{httpbin_base_url}}/get?hello=world\nX-Custom-Header: BeardGit\n",
        ),
        (
            "quickstart/httpbin/post-form.http",
            "# @name POST form-urlencoded\nPOST {{httpbin_base_url}}/post\nContent-Type: application/x-www-form-urlencoded\n\nname=Adolfo&role=admin\n",
        ),
        (
            "quickstart/httpbin/status-code.http",
            "# @name Trigger a 418 I'm a teapot\nGET {{httpbin_base_url}}/status/418\n",
        ),
        (
            "quickstart/httpbin/delay.http",
            "# @name Slow response (good for testing Cancel)\nGET {{httpbin_base_url}}/delay/5\n",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_quickstart_writes_nine_files_and_first_is_list_posts() {
        let dir = tempfile::tempdir().unwrap();
        let written = seed_quickstart_pack(dir.path()).unwrap();
        assert_eq!(
            written.len(),
            9,
            "quickstart pack must seed 5 jsonplaceholder + 4 httpbin samples, got {written:?}",
        );
        // First file is the natural landing target after seeding.
        assert_eq!(
            written[0], "quickstart/jsonplaceholder/list-posts.http",
            "first written path must be list-posts.http so the UI can auto-select it",
        );
        // Spot-check both subfolders ended up on disk.
        for rel in [
            "quickstart/jsonplaceholder/list-posts.http",
            "quickstart/jsonplaceholder/get-post.http",
            "quickstart/jsonplaceholder/create-post.http",
            "quickstart/jsonplaceholder/update-post.http",
            "quickstart/jsonplaceholder/delete-post.http",
            "quickstart/httpbin/inspect-request.http",
            "quickstart/httpbin/post-form.http",
            "quickstart/httpbin/status-code.http",
            "quickstart/httpbin/delay.http",
        ] {
            let p = dir.path().join(".beardgit/requests").join(rel);
            assert!(p.exists(), "missing seeded file: {rel}");
        }
    }

    #[test]
    fn seed_quickstart_populates_default_env_with_matching_vars() {
        let dir = tempfile::tempdir().unwrap();
        seed_quickstart_pack(dir.path()).unwrap();
        let env_path = dir.path().join(".beardgit/requests/_env/default.json");
        let body = std::fs::read_to_string(&env_path).unwrap();
        assert!(
            body.contains("\"base_url\""),
            "default env must define base_url, got: {body}"
        );
        assert!(
            body.contains("https://jsonplaceholder.typicode.com"),
            "default env must point base_url at JSONPlaceholder, got: {body}",
        );
        assert!(
            body.contains("\"httpbin_base_url\""),
            "default env must define httpbin_base_url, got: {body}",
        );
        assert!(
            body.contains("https://httpbin.org"),
            "default env must point httpbin_base_url at httpbin.org, got: {body}",
        );
        assert!(
            body.contains("\"post_id\""),
            "default env must define post_id, got: {body}",
        );
    }

    #[test]
    fn seed_quickstart_overwrites_empty_stub_default_env() {
        // The "Start empty" SeedPrompt path writes an inline stub. When
        // the user then clicks Quickstart, it's safe to upgrade the
        // empty stub to the populated env (since the user has not
        // edited it yet).
        let dir = tempfile::tempdir().unwrap();
        let env_dir = dir.path().join(".beardgit/requests/_env");
        std::fs::create_dir_all(&env_dir).unwrap();
        std::fs::write(env_dir.join("default.json"), EMPTY_DEFAULT_ENV).unwrap();
        seed_quickstart_pack(dir.path()).unwrap();
        let body = std::fs::read_to_string(env_dir.join("default.json")).unwrap();
        assert!(
            body.contains("base_url"),
            "empty stub must be upgraded to the quickstart env, got: {body}",
        );
    }

    #[test]
    fn seed_quickstart_preserves_user_edited_default_env() {
        let dir = tempfile::tempdir().unwrap();
        let env_dir = dir.path().join(".beardgit/requests/_env");
        std::fs::create_dir_all(&env_dir).unwrap();
        let custom =
            "{\"$schema\":\"beardgit-env/v1\",\"vars\":{\"my_var\":\"keep me\"},\"secrets\":[]}";
        std::fs::write(env_dir.join("default.json"), custom).unwrap();
        seed_quickstart_pack(dir.path()).unwrap();
        let body = std::fs::read_to_string(env_dir.join("default.json")).unwrap();
        assert_eq!(
            body, custom,
            "user-customised default.json must not be overwritten",
        );
    }

    #[test]
    fn seed_quickstart_files_have_no_forge_token_references() {
        // The whole point of the Quickstart pack is that it works
        // without any forge auth. Guard against accidentally adding
        // `{{forge_token}}` (or any forge_*) references back in.
        let dir = tempfile::tempdir().unwrap();
        seed_quickstart_pack(dir.path()).unwrap();
        for (rel, _) in quickstart_files() {
            let p = dir.path().join(".beardgit/requests").join(rel);
            let body = std::fs::read_to_string(&p).unwrap();
            assert!(
                !body.contains("forge_"),
                "quickstart file {rel} unexpectedly references a forge_* var: {body}",
            );
        }
    }
}
