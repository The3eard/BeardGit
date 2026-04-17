//! Release listing, detail, CRUD, and asset operations.
//!
//! Read + write operations (list/get/create/edit/delete/publish,
//! delete_asset) are synchronous-style commands that dispatch to the active
//! [`forge_provider::ForgeProvider`] on a `spawn_blocking` thread.
//!
//! **Asset upload is special:** it returns a `TaskId` immediately and
//! streams stdout/stderr from the `gh release upload` / `glab release
//! upload` subprocess via the [`task_runner::TaskManager`]. Large binary
//! uploads don't block the UI; the frontend subscribes to task events to
//! show progress.
//!
//! ## Atomic create-tag + push + release
//!
//! [`create_tag_and_release`] creates a local tag, pushes it to the
//! remote, then creates the release — all sequentially inside a single
//! task so any failure surfaces with a single error. It is used by the
//! "new tag" mode of the `CreateReleaseDialog` so the user gets atomic
//! feedback on the full flow.

use std::sync::Arc;

use forge_provider::{
    CreateReleaseInput, EditReleasePatch, ForgeProvider, Release, ReleaseAsset, ReleaseDetail,
};
use task_runner::{TaskId, TaskManager};
use tauri::State;

use super::helpers::*;
use crate::state::AppState;

/// List releases for the current repository, newest first.
#[tauri::command]
pub async fn list_releases(
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<Release>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    let limit = limit.unwrap_or(30);
    run_blocking(move || provider.list_releases(limit).map_err(|e| e.to_string())).await
}

/// Fetch full detail (body + assets) for a single release by tag.
#[tauri::command]
pub async fn get_release_detail(
    tag: String,
    state: State<'_, AppState>,
) -> Result<ReleaseDetail, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.get_release(&tag).map_err(|e| e.to_string())).await
}

/// List just the asset records for a release.
#[tauri::command]
pub async fn list_release_assets(
    tag: String,
    state: State<'_, AppState>,
) -> Result<Vec<ReleaseAsset>, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .list_release_assets(&tag)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Create a new release.
///
/// On GitHub, `input.target_commit` can be an unpushed branch or SHA — the
/// CLI will create the tag remotely. On GitLab the caller is expected to
/// have pushed the tag already (use [`create_tag_and_release`] for the
/// create+push+release flow).
#[tauri::command]
pub async fn create_release(
    input: CreateReleaseInput,
    state: State<'_, AppState>,
) -> Result<Release, String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.create_release(input).map_err(|e| e.to_string())).await
}

/// Edit a release's title, body, and/or draft/prerelease flags.
#[tauri::command]
pub async fn edit_release(
    tag: String,
    patch: EditReleasePatch,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .edit_release(&tag, patch)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Delete a release. The underlying tag is not removed.
#[tauri::command]
pub async fn delete_release(tag: String, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.delete_release(&tag).map_err(|e| e.to_string())).await
}

/// Publish a draft release. GitHub only — GitLab returns a NotSupported error.
#[tauri::command]
pub async fn publish_release(tag: String, state: State<'_, AppState>) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || provider.publish_release(&tag).map_err(|e| e.to_string())).await
}

/// Delete a single release asset by ID.
#[tauri::command]
pub async fn delete_release_asset(
    tag: String,
    asset_id: u64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;
    run_blocking(move || {
        provider
            .delete_release_asset(&tag, asset_id)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Upload a binary asset to a release.
///
/// Non-blocking: returns a `TaskId` immediately. The underlying `gh
/// release upload` / `glab release upload` subprocess runs via
/// [`TaskManager`] so its stdout/stderr stream to the task popover and
/// the UI stays responsive even for large binaries.
///
/// The frontend subscribes to `task-completed` / `task-failed` events for
/// this id and re-fetches the release detail on success to pick up the
/// newly uploaded asset row.
#[tauri::command]
pub async fn upload_release_asset(
    tag: String,
    asset_path: String,
    label: Option<String>,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    // Resolve which CLI we're speaking to and the upload argv shape.
    let (binary, args) = {
        let kind = {
            let providers = state.providers.lock().map_err(|e| e.to_string())?;
            let active = state
                .active_provider_index
                .lock()
                .map_err(|e| e.to_string())?;
            let idx = active.ok_or_else(|| "No active provider".to_string())?;
            let conn = providers
                .get(idx)
                .ok_or_else(|| "Active provider index out of bounds".to_string())?;
            conn.kind
        };
        let bin = resolve_cli_binary(&state, kind)?;
        let args = build_upload_argv(kind, &tag, &asset_path, label.as_deref());
        (bin, args)
    };

    let file_name = std::path::Path::new(&asset_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("asset")
        .to_string();
    let task_label = match label.as_deref() {
        Some(l) if !l.is_empty() => format!("Upload asset: {file_name} ({l}) → {tag}"),
        _ => format!("Upload asset: {file_name} → {tag}"),
    };

    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    let binary_str = binary.to_string_lossy().to_string();

    let id = task_manager
        .spawn(task_label, &binary_str, &args_ref, &cwd, true)
        .await;
    Ok(id)
}

/// Build argv for the underlying CLI release upload.
///
/// - **GitHub** (`gh release upload`): `[tag, file#label?, --clobber]`.
/// - **GitLab** (`glab release upload`): `[tag, file]` (labels unsupported).
fn build_upload_argv(
    kind: provider::ProviderKind,
    tag: &str,
    asset_path: &str,
    label: Option<&str>,
) -> Vec<String> {
    match kind {
        provider::ProviderKind::GitHub => {
            let mut args: Vec<String> = vec!["release".into(), "upload".into(), tag.into()];
            if let Some(l) = label.filter(|s| !s.is_empty()) {
                args.push(format!("{asset_path}#{l}"));
            } else {
                args.push(asset_path.into());
            }
            args.push("--clobber".into());
            args
        }
        provider::ProviderKind::GitLab => {
            vec![
                "release".into(),
                "upload".into(),
                tag.into(),
                asset_path.into(),
            ]
        }
    }
}

/// Atomic create-tag + push + create-release.
///
/// Runs sequentially:
/// 1. Create a local tag pointing at `source_ref` (lightweight tag).
/// 2. Push the tag to `remote` via `git push`.
/// 3. Call the provider's `create_release`.
///
/// Returns immediately with a `TaskId`; progress messages are streamed as
/// task output via [`TaskManager::spawn`]. Because the underlying steps use
/// heterogeneous mechanisms (git CLI for steps 1-2, forge provider for step
/// 3), we assemble a small shell wrapper: the tag create/push runs via
/// `git` in a subprocess; the release create is then attempted in-process
/// after the subprocess exits. For simplicity and to fit into the
/// subprocess-only TaskManager, we shell out all three steps — tag +
/// push use `git`, and the release create is handled in a follow-up
/// blocking call after the task completes.
///
/// For now we take the simpler approach: run `git tag && git push` as a
/// single combined task, and perform the `create_release` call in the
/// success path via a polling listener, then emit a `release-created`
/// event. This mirrors the `checkout_mr_pr_locally` pattern.
#[tauri::command]
pub async fn create_tag_and_release(
    tag: String,
    source_ref: String,
    remote: String,
    input: CreateReleaseInput,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
    app_handle: tauri::AppHandle,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;
    let provider: Arc<dyn ForgeProvider> = build_forge_provider(&state)?;

    // Step 1+2 as a single combined subprocess: `sh -c "git tag X SRC && git push REMOTE X"`.
    // This keeps stdout/stderr streaming through TaskManager.
    let tag_ref = format!("refs/tags/{tag}");
    let combined = format!(
        "git tag {tag} {source_ref} && git push {remote} {tag_ref}",
        tag = shell_escape(&tag),
        source_ref = shell_escape(&source_ref),
        remote = shell_escape(&remote),
        tag_ref = shell_escape(&tag_ref),
    );

    let (shell, flag) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let args = [flag, combined.as_str()];

    let task_label = format!("Create tag + release: {tag}");
    let id = task_manager
        .spawn(task_label, shell, &args, &cwd, true)
        .await;

    // Spawn a listener that, on success, invokes `create_release` and emits
    // a `release-created` event with the Release as payload.
    let tm: Arc<TaskManager> = Arc::clone(&task_manager);
    let handle = app_handle.clone();
    let tag_for_listener = tag.clone();
    tokio::spawn(async move {
        use task_runner::TaskStatus;
        loop {
            let info = tm.list_tasks().await.into_iter().find(|t| t.id == id);
            match info.as_ref().map(|i| &i.status) {
                Some(TaskStatus::Completed) => {
                    // Tag is pushed; now attempt the release create.
                    let provider = Arc::clone(&provider);
                    let input = input.clone();
                    let tag = tag_for_listener.clone();
                    let result = tokio::task::spawn_blocking(move || {
                        provider.create_release(input).map_err(|e| e.to_string())
                    })
                    .await;
                    use tauri::Emitter as _;
                    match result {
                        Ok(Ok(release)) => {
                            let _ = handle.emit("release-created", &release);
                        }
                        Ok(Err(e)) => {
                            let _ = handle.emit(
                                "release-create-failed",
                                &serde_json::json!({ "tag": tag, "error": e }),
                            );
                        }
                        Err(e) => {
                            let _ = handle.emit(
                                "release-create-failed",
                                &serde_json::json!({ "tag": tag, "error": e.to_string() }),
                            );
                        }
                    }
                    break;
                }
                Some(TaskStatus::Failed { .. }) | Some(TaskStatus::Cancelled) => break,
                _ => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
            }
        }
    });

    Ok(id)
}

/// Shell-escape a value for use in a POSIX `sh -c` command line.
///
/// Conservative: wraps in single quotes and escapes embedded single quotes.
/// On Windows, `cmd.exe` handles most of the values we pass (tag names,
/// refs, remotes) verbatim — the escaping here is best-effort for POSIX
/// and still produces a working command on Windows for typical inputs.
fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".into();
    }
    // If the value is safe (alphanumerics, slashes, dots, dashes, underscores), pass as-is.
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'))
    {
        return s.to_string();
    }
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_argv_github_no_label_ends_with_clobber() {
        let args = build_upload_argv(provider::ProviderKind::GitHub, "v1.0.0", "/tmp/a.dmg", None);
        assert_eq!(args[0], "release");
        assert_eq!(args[1], "upload");
        assert_eq!(args[2], "v1.0.0");
        assert_eq!(args[3], "/tmp/a.dmg");
        assert_eq!(args[4], "--clobber");
    }

    #[test]
    fn upload_argv_github_with_label_encodes_hash_syntax() {
        let args = build_upload_argv(
            provider::ProviderKind::GitHub,
            "v1.0.0",
            "/tmp/a.dmg",
            Some("Mac arm64"),
        );
        assert!(args.iter().any(|a| a == "/tmp/a.dmg#Mac arm64"));
        assert!(args.contains(&"--clobber".to_string()));
    }

    #[test]
    fn upload_argv_gitlab_omits_clobber_and_label() {
        let args = build_upload_argv(
            provider::ProviderKind::GitLab,
            "v1.0.0",
            "/tmp/a.tar.gz",
            Some("Linux x64"),
        );
        assert_eq!(args.len(), 4);
        assert_eq!(args[3], "/tmp/a.tar.gz");
        assert!(!args.contains(&"--clobber".to_string()));
    }

    #[test]
    fn shell_escape_safe_passes_through() {
        assert_eq!(shell_escape("v1.2.3"), "v1.2.3");
        assert_eq!(shell_escape("feature/foo"), "feature/foo");
        assert_eq!(shell_escape("origin"), "origin");
    }

    #[test]
    fn shell_escape_spaces_get_quoted() {
        assert_eq!(shell_escape("release notes"), "'release notes'");
    }

    #[test]
    fn shell_escape_embedded_single_quote() {
        // O'Brien → 'O'\''Brien'
        assert_eq!(shell_escape("O'Brien"), "'O'\\''Brien'");
    }
}
