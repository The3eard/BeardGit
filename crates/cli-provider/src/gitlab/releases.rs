//! Releases for the GitLab CLI provider.
//!
//! Delegates to `glab release *`. Argv-builder and JSON-parser helpers live
//! in the shared [`crate::releases`] module. GitLab has no draft/prerelease
//! concept, so [`publish_release_impl`] returns [`ForgeError::NotSupported`].

use forge_provider::{
    CreateReleaseInput, EditReleasePatch, ForgeError, Release, ReleaseAsset, ReleaseDetail,
};

use super::GitLabCli;
use crate::releases::{
    build_glab_create_args, build_glab_delete_asset_endpoint, build_glab_update_args,
    parse_glab_release_detail, parse_glab_releases,
};

impl GitLabCli {
    pub(super) fn list_releases_impl(&self, limit: u32) -> Result<Vec<Release>, ForgeError> {
        let per_page = limit.to_string();
        let stdout = self.run(&["release", "list", "--per-page", &per_page, "-F", "json"])?;
        parse_glab_releases(&stdout).map_err(|e| ForgeError::Cli(e.to_string()))
    }

    pub(super) fn get_release_impl(&self, tag: &str) -> Result<ReleaseDetail, ForgeError> {
        let stdout = self.run(&["release", "view", tag, "-F", "json"])?;
        parse_glab_release_detail(&stdout).map_err(|e| ForgeError::Cli(e.to_string()))
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
        let args = build_glab_create_args(&input);
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
        Ok(self.get_release_impl(&input.tag)?.summary)
    }

    pub(super) fn edit_release_impl(
        &self,
        tag: &str,
        patch: EditReleasePatch,
    ) -> Result<(), ForgeError> {
        let args = build_glab_update_args(tag, &patch);
        if args.len() == 3 {
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

    // GitLab has no draft/prerelease concept — publish is a no-op / unsupported.
    pub(super) fn publish_release_impl(&self, _tag: &str) -> Result<(), ForgeError> {
        Err(ForgeError::NotSupported)
    }

    pub(super) fn upload_release_asset_impl(
        &self,
        tag: &str,
        path: &std::path::Path,
        label: Option<&str>,
    ) -> Result<ReleaseAsset, ForgeError> {
        let path_str = path.to_string_lossy().to_string();
        // `glab release upload TAG FILE` uploads a single asset. The `--label`
        // flag is not supported in the same way as `gh`; we document this as
        // a known gap (see plan 8.5 Known Gaps) and ignore the label parameter.
        let args = crate::releases::build_glab_upload_args(tag, &path_str, label);
        let ref_args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.run(&ref_args)?;
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
        let endpoint = build_glab_delete_asset_endpoint(tag, asset_id);
        self.run(&["api", &endpoint, "--method", "DELETE"])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitlab_publish_release_not_supported() {
        let p = GitLabCli::new(
            std::path::PathBuf::from("/nonexistent/glab"),
            std::path::PathBuf::from("/tmp"),
        );
        assert!(matches!(
            p.publish_release_impl("v1"),
            Err(ForgeError::NotSupported)
        ));
    }
}
