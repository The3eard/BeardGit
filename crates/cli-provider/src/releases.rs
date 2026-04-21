//! Release parsing + argv builders for GitHub (`gh release`) and GitLab
//! (`glab release`).
//!
//! Parsers consume the JSON emitted by the `--json` / `-F json` flags and
//! map it into the shared [`forge_provider::Release`] / [`ReleaseAsset`] /
//! [`ReleaseDetail`] types. argv builders are pure functions so the command
//! shape can be unit-tested without spawning subprocesses.
//
// DIAGNOSIS 2026-04-21 — "release blank pane" bug
// ------------------------------------------------
// Symptom: some releases render a blank detail pane in the UI.
// Root cause: GitHub returns `"body": null` (and occasionally
// `"assets": null`) for releases where the notes/assets fields were
// left empty by `gh release create --notes ''` or similar flows.
//
// The current `parse_gh_release_detail` relies on `#[serde(default)]`
// on `body: String` / `assets: Vec<GhAssetRow>`. `#[serde(default)]`
// only substitutes a default when the KEY IS MISSING — it does NOT
// accept an explicit JSON `null` value and fails the whole payload
// with a serde decode error. That error surfaces as
// `ForgeError::Cli`; the frontend's catch clears `releaseDetail` to
// null, producing the blank pane.
//
// Fix path (upcoming phases): introduce a `null_as_default` custom
// deserializer that treats both missing keys AND explicit `null` as
// the `Default::default()` value, and apply it to the affected
// fields (`body`, `assets`, and any other String/Vec fields that
// GitHub may emit as null).

use forge_provider::{
    CreateReleaseInput, EditReleasePatch, Release, ReleaseAsset, ReleaseDetail, ReleaseState,
};
use serde::Deserialize;

/// Serde deserializer that maps JSON `null` to the type's `Default` value.
///
/// `gh release view --json body,assets` emits `null` (not an empty
/// string / empty array) when those fields were never set at release
/// creation time — same story for `glab release view -F json` on
/// `description` / `assets`. Plain `#[serde(default)]` only substitutes
/// a default when the KEY IS MISSING; it does not accept explicit null
/// and fails the whole payload, which surfaced as a blank release
/// detail pane in the UI.
///
/// Pair with `#[serde(default, deserialize_with = "null_as_default")]`
/// so both shapes — missing key AND explicit null — degrade to
/// `Default::default()`.
fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + serde::Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

// ─── GitHub (gh) ────────────────────────────────────────────────────────────

/// One row from `gh release list --json ...`.
#[derive(Debug, Deserialize)]
struct GhReleaseRow {
    #[serde(rename = "tagName")]
    tag_name: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "isDraft", default)]
    is_draft: bool,
    #[serde(rename = "isPrerelease", default)]
    is_prerelease: bool,
    #[serde(rename = "publishedAt")]
    published_at: Option<String>,
    #[serde(rename = "createdAt", default)]
    created_at: String,
    #[serde(default)]
    author: GhAuthor,
    #[serde(default)]
    url: String,
}

#[derive(Debug, Default, Deserialize)]
struct GhAuthor {
    #[serde(default)]
    login: String,
}

/// Row from `gh release view --json ...` (summary + body + assets).
///
/// `body` and `assets` use `null_as_default` so releases created with
/// `gh release create --notes ''` (which stores `null`, not `""`) still
/// parse cleanly. Without this the whole payload fails to deserialize
/// and the UI renders a blank detail pane.
#[derive(Debug, Deserialize)]
struct GhReleaseDetailRow {
    #[serde(flatten)]
    summary: GhReleaseRow,
    #[serde(default, deserialize_with = "null_as_default")]
    body: String,
    #[serde(default, deserialize_with = "null_as_default")]
    assets: Vec<GhAssetRow>,
}

#[derive(Debug, Deserialize)]
struct GhAssetRow {
    #[serde(default)]
    id: u64,
    name: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    size: u64,
    #[serde(rename = "downloadCount", default)]
    download_count: u64,
    #[serde(rename = "contentType", default)]
    content_type: String,
    #[serde(default)]
    url: String,
}

fn gh_state(is_draft: bool, is_prerelease: bool) -> ReleaseState {
    if is_draft {
        ReleaseState::Draft
    } else if is_prerelease {
        ReleaseState::Prerelease
    } else {
        ReleaseState::Published
    }
}

fn gh_row_to_release(r: GhReleaseRow, asset_count: u64) -> Release {
    Release {
        state: gh_state(r.is_draft, r.is_prerelease),
        tag: r.tag_name,
        name: r.name,
        author: r.author.login,
        created_at: r.created_at,
        published_at: r.published_at,
        asset_count,
        url: r.url,
    }
}

fn asset_from_gh(a: GhAssetRow) -> ReleaseAsset {
    ReleaseAsset {
        id: a.id,
        name: a.name,
        label: if a.label.is_empty() {
            None
        } else {
            Some(a.label)
        },
        size: a.size,
        download_count: a.download_count,
        content_type: a.content_type,
        url: a.url,
    }
}

/// Parse stdout from `gh release list --json ...` into a list of releases.
///
/// Asset count is not available on list rows (it only appears in the detail
/// view), so entries from list are reported with `asset_count = 0`.
pub(crate) fn parse_gh_releases(json: &str) -> Result<Vec<Release>, serde_json::Error> {
    let rows: Vec<GhReleaseRow> = serde_json::from_str(json)?;
    Ok(rows.into_iter().map(|r| gh_row_to_release(r, 0)).collect())
}

/// Parse stdout from `gh release view --json ...` into a [`ReleaseDetail`].
pub(crate) fn parse_gh_release_detail(json: &str) -> Result<ReleaseDetail, serde_json::Error> {
    let row: GhReleaseDetailRow = serde_json::from_str(json)?;
    let assets: Vec<ReleaseAsset> = row.assets.into_iter().map(asset_from_gh).collect();
    let asset_count = assets.len() as u64;
    let summary = gh_row_to_release(row.summary, asset_count);
    Ok(ReleaseDetail {
        summary,
        body: row.body,
        assets,
    })
}

/// Build argv for `gh release create` from a [`CreateReleaseInput`].
///
/// Extracted into a pure function so the command shape can be asserted in
/// tests without spawning a real `gh` subprocess.
pub(crate) fn build_gh_create_args(input: &CreateReleaseInput) -> Vec<String> {
    let mut args: Vec<String> = vec!["release".into(), "create".into(), input.tag.clone()];
    if !input.target_commit.is_empty() {
        args.push("--target".into());
        args.push(input.target_commit.clone());
    }
    args.push("--title".into());
    args.push(input.name.clone());
    args.push("--notes".into());
    args.push(input.body.clone());
    if input.draft {
        args.push("--draft".into());
    }
    if input.prerelease {
        args.push("--prerelease".into());
    }
    if input.generate_notes {
        args.push("--generate-notes".into());
    }
    args
}

/// Build argv for `gh release edit` from an [`EditReleasePatch`].
///
/// Returns `["release", "edit", tag]` on an empty patch — the caller should
/// treat that as a no-op.
pub(crate) fn build_gh_edit_args(tag: &str, patch: &EditReleasePatch) -> Vec<String> {
    let mut args: Vec<String> = vec!["release".into(), "edit".into(), tag.into()];
    if let Some(name) = &patch.name {
        args.push("--title".into());
        args.push(name.clone());
    }
    if let Some(body) = &patch.body {
        args.push("--notes".into());
        args.push(body.clone());
    }
    if let Some(draft) = patch.draft {
        args.push(format!("--draft={draft}"));
    }
    if let Some(pre) = patch.prerelease {
        args.push(format!("--prerelease={pre}"));
    }
    args
}

/// Build argv for `gh release upload` — file with optional `#label` suffix.
pub fn build_gh_upload_args(tag: &str, file: &str, label: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = vec!["release".into(), "upload".into(), tag.into()];
    if let Some(label) = label {
        args.push(format!("{file}#{label}"));
    } else {
        args.push(file.into());
    }
    args.push("--clobber".into());
    args
}

/// Build argv for `glab release upload` — label is unsupported by glab
/// (documented in `gitlab/releases.rs::upload_release_asset_impl`), so
/// the parameter is accepted for call-site symmetry and silently ignored.
pub fn build_glab_upload_args(tag: &str, file: &str, _label: Option<&str>) -> Vec<String> {
    vec!["release".into(), "upload".into(), tag.into(), file.into()]
}

// ─── GitLab (glab) ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GlabReleaseRow {
    tag_name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    description: String,
    #[serde(default, deserialize_with = "null_as_default")]
    created_at: String,
    released_at: Option<String>,
    #[serde(default)]
    upcoming_release: bool,
    #[serde(default)]
    author: GlabAuthor,
    #[serde(default, rename = "_links")]
    links: GlabLinks,
    #[serde(default, deserialize_with = "null_as_default")]
    assets: GlabAssets,
}

#[derive(Debug, Default, Deserialize)]
struct GlabAuthor {
    #[serde(default)]
    username: String,
}

#[derive(Debug, Default, Deserialize)]
struct GlabLinks {
    #[serde(default, rename = "self")]
    self_url: String,
}

#[derive(Debug, Default, Deserialize)]
struct GlabAssets {
    #[serde(default)]
    count: u64,
    #[serde(default, deserialize_with = "null_as_default")]
    links: Vec<GlabAssetLink>,
}

#[derive(Debug, Clone, Deserialize)]
struct GlabAssetLink {
    id: u64,
    name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    link_type: String,
}

fn gl_row_to_release(r: &GlabReleaseRow) -> Release {
    Release {
        tag: r.tag_name.clone(),
        name: r.name.clone(),
        state: if r.upcoming_release {
            ReleaseState::Prerelease
        } else {
            ReleaseState::Published
        },
        author: r.author.username.clone(),
        created_at: r.created_at.clone(),
        published_at: r.released_at.clone(),
        asset_count: r.assets.count,
        url: r.links.self_url.clone(),
    }
}

/// Parse stdout from `glab release list -F json` into a list of releases.
pub(crate) fn parse_glab_releases(json: &str) -> Result<Vec<Release>, serde_json::Error> {
    let rows: Vec<GlabReleaseRow> = serde_json::from_str(json)?;
    Ok(rows.iter().map(gl_row_to_release).collect())
}

/// Parse stdout from `glab release view TAG -F json` into a [`ReleaseDetail`].
pub(crate) fn parse_glab_release_detail(json: &str) -> Result<ReleaseDetail, serde_json::Error> {
    let row: GlabReleaseRow = serde_json::from_str(json)?;
    let body = row.description.clone();
    let assets: Vec<ReleaseAsset> = row
        .assets
        .links
        .iter()
        .cloned()
        .map(|l| ReleaseAsset {
            id: l.id,
            name: l.name,
            label: None,
            size: 0,
            download_count: 0,
            content_type: l.link_type,
            url: l.url,
        })
        .collect();
    let summary = gl_row_to_release(&row);
    Ok(ReleaseDetail {
        summary,
        body,
        assets,
    })
}

/// Build argv for `glab release create`.
pub(crate) fn build_glab_create_args(input: &CreateReleaseInput) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "release".into(),
        "create".into(),
        input.tag.clone(),
        "--name".into(),
        input.name.clone(),
        "--notes".into(),
        input.body.clone(),
    ];
    if !input.target_commit.is_empty() {
        args.push("--ref".into());
        args.push(input.target_commit.clone());
    }
    args
}

/// Build argv for `glab release update`.
///
/// Returns `["release", "update", tag]` on a no-op patch. `draft` and
/// `prerelease` are silently ignored (GitLab has no such concepts).
pub(crate) fn build_glab_update_args(tag: &str, patch: &EditReleasePatch) -> Vec<String> {
    let mut args: Vec<String> = vec!["release".into(), "update".into(), tag.into()];
    if let Some(name) = &patch.name {
        args.push("--name".into());
        args.push(name.clone());
    }
    if let Some(body) = &patch.body {
        args.push("--notes".into());
        args.push(body.clone());
    }
    args
}

/// Build a `glab api` endpoint path for deleting a single release asset link.
pub(crate) fn build_glab_delete_asset_endpoint(tag: &str, asset_id: u64) -> String {
    format!("projects/:fullpath/releases/{tag}/assets/links/{asset_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── GitHub parser tests ────────────────────────────────────────────

    #[test]
    fn parses_gh_release_list_fixture() {
        let json = include_str!("../tests/fixtures/gh_release_list.json");
        let releases = parse_gh_releases(json).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].tag, "v0.1.8");
        assert_eq!(releases[0].state, ReleaseState::Published);
        assert_eq!(releases[0].author, "adolfo");
    }

    #[test]
    fn parses_gh_release_view_fixture() {
        let json = include_str!("../tests/fixtures/gh_release_view.json");
        let detail = parse_gh_release_detail(json).unwrap();
        assert_eq!(detail.summary.tag, "v0.1.8");
        assert_eq!(detail.assets.len(), 1);
        assert_eq!(detail.assets[0].name, "beardgit-mac-arm64.dmg");
        assert_eq!(detail.summary.asset_count, 1);
    }

    #[test]
    fn parses_gh_release_view_with_null_body() {
        // Regression test for the "release blank pane" bug: `gh release
        // view --json body,…` emits `"body": null` when the release was
        // created with empty notes. `#[serde(default)]` alone rejects
        // explicit null and fails the whole payload, so the detail pane
        // rendered blank. With `null_as_default`, null must degrade to
        // an empty string and the rest of the payload must parse.
        let json = include_str!("../tests/fixtures/gh_release_view_null_body.json");
        let detail = parse_gh_release_detail(json).unwrap();
        assert_eq!(detail.body, "");
        assert_eq!(detail.summary.tag, "v0.1.8");
        assert_eq!(detail.assets.len(), 1);
    }

    #[test]
    fn parses_gh_release_view_with_null_assets() {
        // Same story for `"assets": null` — must degrade to an empty
        // `Vec<ReleaseAsset>` instead of failing the whole parse.
        let json = include_str!("../tests/fixtures/gh_release_view_null_assets.json");
        let detail = parse_gh_release_detail(json).unwrap();
        assert!(detail.assets.is_empty());
        assert_eq!(detail.summary.asset_count, 0);
        assert_eq!(detail.summary.tag, "v0.1.8");
    }

    #[test]
    fn gh_draft_release_maps_to_draft_state() {
        let json = r#"[{"tagName":"v1","name":"","isDraft":true,"isPrerelease":false,"publishedAt":null,"createdAt":"","author":{"login":"a"},"url":""}]"#;
        let r = parse_gh_releases(json).unwrap();
        assert_eq!(r[0].state, ReleaseState::Draft);
    }

    #[test]
    fn gh_prerelease_maps_to_prerelease_state() {
        let json = r#"[{"tagName":"v1","name":"","isDraft":false,"isPrerelease":true,"publishedAt":"","createdAt":"","author":{"login":"a"},"url":""}]"#;
        let r = parse_gh_releases(json).unwrap();
        assert_eq!(r[0].state, ReleaseState::Prerelease);
    }

    #[test]
    fn gh_empty_label_becomes_none() {
        let json = r#"{"tagName":"v1","name":"","isDraft":false,"isPrerelease":false,"publishedAt":"","createdAt":"","author":{"login":"a"},"url":"","body":"","assets":[{"id":1,"name":"f","label":"","size":1,"downloadCount":0,"contentType":"x","url":"u"}]}"#;
        let d = parse_gh_release_detail(json).unwrap();
        assert!(d.assets[0].label.is_none());
    }

    #[test]
    fn gh_nonempty_label_is_some() {
        let json = r#"{"tagName":"v1","name":"","isDraft":false,"isPrerelease":false,"publishedAt":"","createdAt":"","author":{"login":"a"},"url":"","body":"","assets":[{"id":1,"name":"f","label":"Mac arm64","size":1,"downloadCount":0,"contentType":"x","url":"u"}]}"#;
        let d = parse_gh_release_detail(json).unwrap();
        assert_eq!(d.assets[0].label.as_deref(), Some("Mac arm64"));
    }

    // ─── GitHub argv tests ──────────────────────────────────────────────

    #[test]
    fn create_draft_prerelease_with_target_builds_expected_argv() {
        let input = CreateReleaseInput {
            tag: "v1.0.0".into(),
            target_commit: "main".into(),
            name: "Release 1.0".into(),
            body: "notes".into(),
            draft: true,
            prerelease: true,
            generate_notes: false,
        };
        let args = build_gh_create_args(&input);
        assert_eq!(args[0], "release");
        assert_eq!(args[1], "create");
        assert_eq!(args[2], "v1.0.0");
        assert!(args.contains(&"--draft".to_string()));
        assert!(args.contains(&"--prerelease".to_string()));
        assert!(args.contains(&"--target".to_string()));
        assert!(args.windows(2).any(|w| w == ["--target", "main"]));
        assert!(args.windows(2).any(|w| w == ["--title", "Release 1.0"]));
        assert!(args.windows(2).any(|w| w == ["--notes", "notes"]));
        assert!(!args.contains(&"--generate-notes".to_string()));
    }

    #[test]
    fn create_without_target_omits_target_flag() {
        let input = CreateReleaseInput {
            tag: "v1.0.0".into(),
            target_commit: "".into(),
            name: "R".into(),
            body: "b".into(),
            draft: false,
            prerelease: false,
            generate_notes: true,
        };
        let args = build_gh_create_args(&input);
        assert!(!args.contains(&"--target".to_string()));
        assert!(args.contains(&"--generate-notes".to_string()));
    }

    #[test]
    fn edit_patch_body_only_omits_other_flags() {
        let p = EditReleasePatch {
            body: Some("new body".into()),
            ..Default::default()
        };
        let args = build_gh_edit_args("v1", &p);
        assert!(!args.contains(&"--title".to_string()));
        assert!(args.windows(2).any(|w| w == ["--notes", "new body"]));
        assert!(!args.iter().any(|a| a.starts_with("--draft")));
        assert!(!args.iter().any(|a| a.starts_with("--prerelease")));
    }

    #[test]
    fn edit_patch_draft_and_prerelease_flags_use_equals_syntax() {
        let p = EditReleasePatch {
            draft: Some(false),
            prerelease: Some(true),
            ..Default::default()
        };
        let args = build_gh_edit_args("v1", &p);
        assert!(args.iter().any(|a| a == "--draft=false"));
        assert!(args.iter().any(|a| a == "--prerelease=true"));
    }

    #[test]
    fn empty_edit_patch_argv_length_is_three() {
        let p = EditReleasePatch::default();
        let args = build_gh_edit_args("v1", &p);
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn upload_with_label_encodes_file_hash_label() {
        let args = build_gh_upload_args("v1.0.0", "/tmp/a.dmg", Some("Mac arm64"));
        assert!(args.iter().any(|a| a == "/tmp/a.dmg#Mac arm64"));
        assert!(args.contains(&"--clobber".to_string()));
    }

    #[test]
    fn upload_without_label_uses_bare_path() {
        let args = build_gh_upload_args("v1.0.0", "/tmp/a.dmg", None);
        assert!(args.iter().any(|a| a == "/tmp/a.dmg"));
        assert!(!args.iter().any(|a| a.contains('#')));
    }

    // ─── GitLab parser tests ────────────────────────────────────────────

    #[test]
    fn parses_glab_release_list_fixture() {
        let json = include_str!("../tests/fixtures/glab_release_list.json");
        let rows = parse_glab_releases(json).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tag, "v0.1.8");
        assert_eq!(rows[0].asset_count, 2);
    }

    #[test]
    fn parses_glab_release_view_fixture() {
        let json = include_str!("../tests/fixtures/glab_release_view.json");
        let detail = parse_glab_release_detail(json).unwrap();
        assert_eq!(detail.assets.len(), 1);
        assert_eq!(detail.assets[0].name, "beardgit-linux-x64.tar.gz");
    }

    #[test]
    fn glab_upcoming_release_maps_to_prerelease_state() {
        let json = r#"[{"tag_name":"v1","name":"","description":"","created_at":"","released_at":null,"upcoming_release":true,"author":{"username":"a"},"_links":{"self":""},"assets":{"count":0,"links":[]}}]"#;
        let r = parse_glab_releases(json).unwrap();
        assert_eq!(r[0].state, ReleaseState::Prerelease);
    }

    // ─── GitLab argv tests ──────────────────────────────────────────────

    #[test]
    fn glab_create_with_target_uses_ref_flag() {
        let input = CreateReleaseInput {
            tag: "v1".into(),
            target_commit: "main".into(),
            name: "R".into(),
            body: "b".into(),
            draft: false,
            prerelease: false,
            generate_notes: false,
        };
        let args = build_glab_create_args(&input);
        assert!(args.windows(2).any(|w| w == ["--ref", "main"]));
        assert!(args.windows(2).any(|w| w == ["--name", "R"]));
        assert!(args.windows(2).any(|w| w == ["--notes", "b"]));
    }

    #[test]
    fn glab_create_without_target_omits_ref_flag() {
        let input = CreateReleaseInput {
            tag: "v1".into(),
            target_commit: "".into(),
            name: "R".into(),
            body: "b".into(),
            draft: false,
            prerelease: false,
            generate_notes: false,
        };
        let args = build_glab_create_args(&input);
        assert!(!args.contains(&"--ref".to_string()));
    }

    #[test]
    fn glab_update_noop_patch_returns_short_argv() {
        let args = build_glab_update_args("v1", &EditReleasePatch::default());
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn glab_update_ignores_draft_and_prerelease_fields() {
        let p = EditReleasePatch {
            name: Some("n".into()),
            draft: Some(true),
            prerelease: Some(true),
            ..Default::default()
        };
        let args = build_glab_update_args("v1", &p);
        assert!(!args.iter().any(|a| a.contains("draft")));
        assert!(!args.iter().any(|a| a.contains("prerelease")));
        assert!(args.windows(2).any(|w| w == ["--name", "n"]));
    }

    #[test]
    fn glab_delete_asset_endpoint_includes_tag_and_id() {
        let endpoint = build_glab_delete_asset_endpoint("v1.0", 42);
        assert!(endpoint.contains("v1.0"));
        assert!(endpoint.contains("42"));
        assert!(endpoint.starts_with("projects/:fullpath/releases/"));
    }

    #[test]
    fn build_glab_upload_args_ignores_label() {
        let args = build_glab_upload_args("v1.0.0", "/tmp/a.dmg", Some("Mac arm64"));
        assert_eq!(args, vec!["release", "upload", "v1.0.0", "/tmp/a.dmg"]);
    }
}
