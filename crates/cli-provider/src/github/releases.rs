//! Releases for the GitHub CLI provider.
//!
//! Delegates to `gh release *`. The actual argv-builder and JSON-parser
//! helpers live in the shared [`crate::releases`] module because several of
//! them share fixture tests with the GitLab builders.

use forge_provider::{
    CreateReleaseInput, EditReleasePatch, ForgeError, Release, ReleaseAsset, ReleaseDetail,
};

use super::GitHubCli;
use crate::releases::{
    build_gh_create_args, build_gh_edit_args, build_gh_upload_args, parse_gh_release_detail,
    parse_gh_releases,
};

impl GitHubCli {
    pub(super) fn list_releases_impl(&self, limit: u32) -> Result<Vec<Release>, ForgeError> {
        let limit_str = limit.to_string();
        let stdout = self.run(&[
            "release",
            "list",
            "-L",
            &limit_str,
            "--json",
            "tagName,name,isDraft,isPrerelease,publishedAt,createdAt,author,url",
        ])?;
        parse_gh_releases(&stdout).map_err(|e| ForgeError::Cli(e.to_string()))
    }

    pub(super) fn get_release_impl(&self, tag: &str) -> Result<ReleaseDetail, ForgeError> {
        let stdout = self.run(&[
            "release",
            "view",
            tag,
            "--json",
            "tagName,name,isDraft,isPrerelease,publishedAt,createdAt,author,url,body,assets",
        ])?;
        parse_gh_release_detail(&stdout).map_err(|e| ForgeError::Cli(e.to_string()))
    }

    pub(super) fn list_release_assets_impl(
        &self,
        tag: &str,
    ) -> Result<Vec<ReleaseAsset>, ForgeError> {
        Ok(self.get_release_impl(tag)?.assets)
    }

    pub(super) fn create_release_impl(
        &self,
        input: CreateReleaseInput,
    ) -> Result<Release, ForgeError> {
        let args = build_gh_create_args(&input);
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
        // `gh release create` prints the release URL, not JSON. Re-fetch the
        // detail view to build a full summary.
        Ok(self.get_release_impl(&input.tag)?.summary)
    }

    pub(super) fn edit_release_impl(
        &self,
        tag: &str,
        patch: EditReleasePatch,
    ) -> Result<(), ForgeError> {
        let args = build_gh_edit_args(tag, &patch);
        if args.len() == 3 {
            // No-op patch — avoid an unnecessary CLI call.
            return Ok(());
        }
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
        Ok(())
    }

    pub(super) fn delete_release_impl(&self, tag: &str) -> Result<(), ForgeError> {
        self.run(&["release", "delete", tag, "--yes"])?;
        Ok(())
    }

    pub(super) fn publish_release_impl(&self, tag: &str) -> Result<(), ForgeError> {
        self.run(&["release", "edit", tag, "--draft=false"])?;
        Ok(())
    }

    pub(super) fn upload_release_asset_impl(
        &self,
        tag: &str,
        path: &std::path::Path,
        label: Option<&str>,
    ) -> Result<ReleaseAsset, ForgeError> {
        let path_str = path.to_string_lossy().to_string();
        let args = build_gh_upload_args(tag, &path_str, label);
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
        // `gh release upload` prints no JSON — re-fetch detail and locate the
        // newly uploaded asset by file name.
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let detail = self.get_release_impl(tag)?;
        detail
            .assets
            .into_iter()
            .find(|a| a.name == name)
            .ok_or_else(|| ForgeError::NotFound(format!("asset {name} after upload")))
    }

    pub(super) fn delete_release_asset_impl(
        &self,
        tag: &str,
        asset_id: u64,
    ) -> Result<(), ForgeError> {
        // `gh release delete-asset` identifies assets by name, not ID. Look up
        // the name from the detail view first.
        let detail = self.get_release_impl(tag)?;
        let name = detail
            .assets
            .iter()
            .find(|a| a.id == asset_id)
            .map(|a| a.name.clone())
            .ok_or_else(|| ForgeError::NotFound(format!("asset id {asset_id}")))?;
        self.run(&["release", "delete-asset", tag, &name, "--yes"])?;
        Ok(())
    }
}
