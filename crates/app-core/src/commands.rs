//! Tauri command handlers exposed to the Svelte frontend via IPC.
//!
//! Commands are grouped into three areas:
//! - **Git** – repository, graph, staging, branches, diffs, and stash operations.
//! - **Provider auth** – connecting, disconnecting, and auto-reconnecting to GitLab or GitHub.
//! - **CI runs** – listing CI runs, fetching run detail, and streaming job logs.

use std::path::PathBuf;
use std::sync::Arc;

use rayon::prelude::*;

use graph_builder::{Dag, GraphCommit, GraphLayout};
use task_runner::{TaskId, TaskManager};
use tauri::{AppHandle, Emitter, State};

use crate::state::{AppState, ProjectSlot};

/// Basic repository metadata returned by [`open_repo`].
#[derive(serde::Serialize)]
pub struct RepoInfo {
    /// Absolute path to the repository root.
    pub path: String,
    /// Name of the currently checked-out branch, if any.
    pub head_branch: Option<String>,
    /// SHA of the HEAD commit, if any.
    pub head_oid: Option<String>,
    /// Total number of local branches.
    pub branch_count: usize,
}

/// A slice of the commit graph used for virtual-scroll rendering.
#[derive(serde::Serialize)]
pub struct GraphViewport {
    pub nodes: Vec<graph_builder::LayoutNode>,
    pub lane_segments: Vec<graph_builder::LaneSegment>,
    pub merge_curves: Vec<graph_builder::MergeCurve>,
    pub total_count: usize,
    pub offset: usize,
    pub visible_lane_count: usize,
    pub total_lane_count: usize,
    /// Lane index of the HEAD commit, if present in the graph.
    pub head_lane: Option<usize>,
}

/// Lightweight project info for tab display (no graph data).
#[derive(serde::Serialize)]
pub struct ProjectInfo {
    /// Absolute filesystem path to the repository root.
    pub path: String,
    /// Repository name (last path segment).
    pub name: String,
    /// Current HEAD branch name, if any.
    pub head_branch: Option<String>,
    /// Number of uncommitted changes.
    pub change_count: usize,
}

/// A recently closed repo for the "+" dropdown.
#[derive(serde::Serialize)]
pub struct RecentRepo {
    /// Absolute filesystem path to the repository root.
    pub path: String,
    /// Repository name (last path segment).
    pub name: String,
}

/// Info about a configured git remote.
#[derive(serde::Serialize)]
pub struct RemoteInfo {
    /// Remote name (e.g. `"origin"`).
    pub name: String,
    /// Remote URL, if available.
    pub url: Option<String>,
}

/// Execute a function with a reference to the active project's repository.
///
/// Locks `projects` and `active_index`, resolves the active [`ProjectSlot`],
/// and calls `f` with the loaded [`git_engine::Repository`]. Returns an error
/// string if no project is active, the index is out of bounds, or no repository
/// is loaded in the slot.
fn with_active_repo<F, R>(state: &State<'_, AppState>, f: F) -> Result<R, String>
where
    F: FnOnce(&git_engine::Repository) -> Result<R, String>,
{
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let active = state.active_index.lock().map_err(|e| e.to_string())?;
    let idx = active.ok_or_else(|| "No active project".to_string())?;
    let slot = projects
        .get(idx)
        .ok_or_else(|| "Active project index out of bounds".to_string())?;
    let repo = slot
        .repo
        .as_ref()
        .ok_or_else(|| "No repository open".to_string())?;
    f(repo)
}

/// Open a git repository at `path`, build the full commit DAG, and store the
/// result in [`AppState`] as a [`ProjectSlot`].
///
/// The heavy graph computation (commit walk, DAG build, layout) runs on a
/// blocking thread so the UI remains responsive on large repositories.
///
/// If the path is already open in an existing slot, that slot is replaced with
/// the freshly loaded data and made active. Otherwise a new slot is appended
/// and made active.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the repository root.
///
/// # Returns
/// [`RepoInfo`] with HEAD branch, HEAD OID, and branch count on success, or an
/// error string if the path is not a valid git repository.
#[tauri::command]
pub async fn open_repo(
    path: String,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<RepoInfo, String> {
    let path_clone = path.clone();

    // Run the expensive graph computation off the main thread
    let (repo, layout, status) = tokio::task::spawn_blocking(move || {
        let repo =
            git_engine::Repository::open(PathBuf::from(&path_clone)).map_err(|e| e.to_string())?;

        let commits = repo.walk_commits(50_000).map_err(|e| e.to_string())?;

        let graph_commits: Vec<GraphCommit> = commits
            .iter()
            .map(|c| GraphCommit {
                oid: c.oid.clone(),
                parents: c.parents.clone(),
                timestamp: c.timestamp,
                refs: c.refs.clone(),
                summary: c.summary.clone(),
                author: c.author.clone(),
                email: c.email.clone(),
            })
            .collect();

        let dag = Dag::build(&graph_commits);
        let layout = GraphLayout::compute(&dag);
        let status = repo.status().map_err(|e| e.to_string())?;

        Ok::<_, String>((repo, layout, status))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: String| e)?;

    // Start filesystem watcher for the new repo (emits `repo-changed` events)
    let repo_path = PathBuf::from(&path);
    let handle = app_handle.clone();
    let new_watcher = watcher::RepoWatcher::start(&repo_path, move || {
        let _ = handle.emit("repo-changed", ());
    })
    .ok();

    // Derive lightweight metadata
    let name = PathBuf::from(&path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    let head_branch = status.head_branch.clone();
    let change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);

    let slot = ProjectSlot {
        path: path.clone(),
        name,
        repo: Some(repo),
        layout: Some(layout),
        watcher: new_watcher,
        head_branch,
        change_count,
    };

    // Insert or replace slot, then set as active
    let active_idx = {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        if let Some(pos) = projects.iter().position(|s| s.path == path) {
            projects[pos] = slot;
            pos
        } else {
            projects.push(slot);
            projects.len() - 1
        }
    };
    *state.active_index.lock().map_err(|e| e.to_string())? = Some(active_idx);

    Ok(RepoInfo {
        path: status.path,
        head_branch: status.head_branch,
        head_oid: status.head_oid,
        branch_count: status.branch_count,
    })
}

/// Return a paginated slice of the commit graph for virtual-scroll rendering.
///
/// # Parameters
/// - `offset` – Zero-based row index of the first commit to include.
/// - `limit`  – Maximum number of rows to return.
///
/// # Returns
/// A [`GraphViewport`] containing the layout nodes for the requested window.
#[tauri::command]
pub async fn get_graph_viewport(
    offset: usize,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<GraphViewport, String> {
    // Extract the viewport result while holding the lock briefly.
    // The layout itself is not Clone/Send, so we compute the viewport
    // slice synchronously (it's array filtering, not DAG computation).
    let (result, total_lane_count) = {
        let projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        let idx = active.ok_or_else(|| "No active project".to_string())?;
        let slot = projects
            .get(idx)
            .ok_or("Active project index out of bounds")?;
        let layout = slot.layout.as_ref().ok_or("No repository open")?;
        let total_lane_count = layout.lane_count;
        let result = layout.viewport(offset, limit);
        (result, total_lane_count)
    };

    Ok(GraphViewport {
        nodes: result.nodes,
        lane_segments: result.lane_segments,
        merge_curves: result.merge_curves,
        total_count: result.total_count,
        offset: result.offset,
        visible_lane_count: result.visible_lane_count,
        total_lane_count,
        head_lane: result.head_lane,
    })
}

/// Look up a commit's row index in the cached graph layout.
///
/// Returns `None` if the commit is not found in the currently loaded graph.
/// This is used by the frontend to scroll the graph viewport to a specific
/// commit (e.g. when navigating from a clickable parent OID).
#[tauri::command]
pub fn get_commit_row(oid: String, state: State<'_, AppState>) -> Result<Option<usize>, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let active = state.active_index.lock().map_err(|e| e.to_string())?;
    let idx = active.ok_or_else(|| "No active project".to_string())?;
    let slot = projects
        .get(idx)
        .ok_or("Active project index out of bounds")?;
    let layout = slot.layout.as_ref().ok_or("No repository open")?;
    Ok(layout.nodes.iter().find(|n| n.oid == oid).map(|n| n.row))
}

/// Search commits using server-side filters and return a graph viewport.
///
/// All filters are optional and combined with AND semantics on the Rust side
/// via `walk_commits_filtered`. Returns a full-viewport result (offset = 0)
/// because the filtered set is typically small enough to display at once.
///
/// # Parameters
/// - `branch`    – Only include commits reachable from this branch.
/// - `author`    – Substring match against the commit author name.
/// - `message`   – Substring match against the commit summary.
/// - `sha`       – Prefix match against the commit OID.
/// - `max_count` – Upper bound on results (defaults to 200).
#[tauri::command]
pub async fn search_commits(
    branch: Option<String>,
    author: Option<String>,
    message: Option<String>,
    sha: Option<String>,
    max_count: Option<usize>,
    state: State<'_, AppState>,
) -> Result<GraphViewport, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;

        let commits = repo
            .walk_commits_filtered(
                max_count.unwrap_or(200),
                branch.as_deref(),
                author.as_deref(),
                message.as_deref(),
                sha.as_deref(),
            )
            .map_err(|e| e.to_string())?;

        // Build graph from filtered commits
        let graph_commits: Vec<GraphCommit> = commits
            .iter()
            .map(|c| GraphCommit {
                oid: c.oid.clone(),
                parents: c.parents.clone(),
                timestamp: c.timestamp,
                refs: c.refs.clone(),
                summary: c.summary.clone(),
                author: c.author.clone(),
                email: c.email.clone(),
            })
            .collect();

        let dag = Dag::build(&graph_commits);
        let layout = GraphLayout::compute(&dag);
        let vp = layout.viewport(0, graph_commits.len());

        Ok(GraphViewport {
            nodes: vp.nodes,
            lane_segments: vp.lane_segments,
            merge_curves: vp.merge_curves,
            total_count: vp.total_count,
            offset: 0,
            visible_lane_count: vp.visible_lane_count,
            total_lane_count: layout.lane_count,
            head_lane: vp.head_lane,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Fetch full metadata for a single commit by its OID.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
///
/// # Returns
/// [`git_engine::CommitInfo`] with author, message, parents, and diff stats.
#[tauri::command]
pub fn get_commit_detail(
    oid: String,
    state: State<'_, AppState>,
) -> Result<git_engine::CommitInfo, String> {
    with_active_repo(&state, |repo| {
        repo.get_commit(&oid).map_err(|e| e.to_string())
    })
}

/// Starship-style status summary for the title bar.
#[tauri::command]
pub fn get_status_summary(state: State<'_, AppState>) -> Result<git_engine::StatusSummary, String> {
    with_active_repo(&state, |repo| {
        repo.status_summary().map_err(|e| e.to_string())
    })
}

/// List files changed by a commit, including their change status and patch.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
#[tauri::command]
pub fn get_commit_files(
    oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    with_active_repo(&state, |repo| {
        repo.commit_files(&oid).map_err(|e| e.to_string())
    })
}

/// Return files changed between two arbitrary commits.
///
/// # Parameters
/// - `from_oid` – SHA of the base commit.
/// - `to_oid` – SHA of the target commit.
#[tauri::command]
pub fn get_diff_between_commits(
    from_oid: String,
    to_oid: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitFileChange>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_commits(&from_oid, &to_oid)
            .map_err(|e| e.to_string())
    })
}

/// Return the full diff (hunks + lines) for a single file in a commit.
#[tauri::command]
pub fn get_commit_file_diff(
    oid: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        repo.commit_file_diff(&oid, &path)
            .map_err(|e| e.to_string())
    })
}

/// Returns raw file content at a specific commit.
///
/// # Parameters
/// - `oid` – Full or abbreviated commit SHA.
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw UTF-8 file content (binary blobs are lossy-decoded), or an error string
/// if the OID or path is invalid.
#[tauri::command]
pub fn get_file_at_commit(
    oid: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_at_commit(&oid, &path)
            .map_err(|e| e.to_string())
    })
}

/// Returns raw file content from the working directory.
///
/// # Parameters
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw file content, or an IO error string if the file does not exist.
#[tauri::command]
pub fn get_file_workdir(path: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_workdir(&path).map_err(|e| e.to_string())
    })
}

/// Returns raw file content from the index (staged version).
///
/// # Parameters
/// - `path` – Repo-relative file path.
///
/// # Returns
/// Raw staged file content, or an error string if the file is not staged.
#[tauri::command]
pub fn get_file_index(path: String, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_file_index(&path).map_err(|e| e.to_string())
    })
}

/// List all local branches in the open repository with their HEAD OIDs.
#[tauri::command]
pub fn get_branches(state: State<'_, AppState>) -> Result<Vec<git_engine::BranchInfo>, String> {
    with_active_repo(&state, |repo| repo.branches().map_err(|e| e.to_string()))
}

/// Return the last N commits on a specific branch.
#[tauri::command]
pub fn get_branch_commits(
    branch_name: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::CommitInfo>, String> {
    with_active_repo(&state, |repo| {
        repo.branch_commits(&branch_name, limit as usize)
            .map_err(|e| e.to_string())
    })
}

/// Return the working-tree and index status for every changed file.
///
/// Used to populate the staging area panel in the UI.
#[tauri::command]
pub fn get_file_statuses(
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileStatus>, String> {
    with_active_repo(&state, |repo| {
        repo.file_statuses().map_err(|e| e.to_string())
    })
}

/// Stage a specific list of files by path (equivalent to `git add <paths>`).
///
/// # Parameters
/// - `paths` – Workspace-relative paths to stage.
#[tauri::command]
pub fn stage_files(paths: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.stage_files(&paths).map_err(|e| e.to_string())
    })
}

/// Unstage a specific list of files (equivalent to `git restore --staged <paths>`).
///
/// # Parameters
/// - `paths` – Workspace-relative paths to unstage.
#[tauri::command]
pub fn unstage_files(paths: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.unstage_files(&paths).map_err(|e| e.to_string())
    })
}

/// Stage all modified and untracked files (equivalent to `git add -A`).
#[tauri::command]
pub fn stage_all(state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| repo.stage_all().map_err(|e| e.to_string()))
}

/// Unstage all staged changes (equivalent to `git restore --staged .`).
#[tauri::command]
pub fn unstage_all(state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| repo.unstage_all().map_err(|e| e.to_string()))
}

/// Create a new commit from the current index with the given message and author.
///
/// # Parameters
/// - `message` – Commit message (subject + optional body).
/// - `name`    – Author display name.
/// - `email`   – Author email address.
///
/// # Returns
/// The OID of the newly created commit as a hex string.
#[tauri::command]
pub fn create_commit(
    message: String,
    name: String,
    email: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.create_commit(&message, &name, &email)
            .map_err(|e| e.to_string())
    })
}

/// Create a new local branch pointing at the current HEAD.
///
/// # Parameters
/// - `name` – Name for the new branch.
#[tauri::command]
pub fn create_branch(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.create_branch(&name).map_err(|e| e.to_string())
    })
}

/// Delete a local branch by name.
///
/// # Parameters
/// - `name` – Name of the branch to delete.
#[tauri::command]
pub fn delete_branch(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.delete_branch(&name).map_err(|e| e.to_string())
    })
}

/// Switch the working tree to an existing local branch.
///
/// # Parameters
/// - `name` – Name of the branch to check out.
#[tauri::command]
pub fn checkout_branch(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.checkout_branch(&name).map_err(|e| e.to_string())
    })
}

/// Return the unstaged diff between the working tree and the index.
///
/// Equivalent to `git diff` (without `--cached`).
#[tauri::command]
pub fn get_diff_workdir(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        repo.diff_workdir().map_err(|e| e.to_string())
    })
}

/// Return the staged diff between the index and HEAD.
///
/// Equivalent to `git diff --cached`.
#[tauri::command]
pub fn get_diff_index(state: State<'_, AppState>) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| repo.diff_index().map_err(|e| e.to_string()))
}

/// Merge a branch into the current branch via the git CLI.
///
/// # Parameters
/// - `branch` – Name of the branch to merge into HEAD.
///
/// # Returns
/// The stdout of `git merge` on success, or stderr as an error.
#[tauri::command]
pub async fn merge_branch(branch: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.merge_branch(&branch).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Cherry-pick a commit onto the current branch via the git CLI.
///
/// # Parameters
/// Revert a commit, creating a new commit that undoes its changes.
///
/// # Arguments
/// - `oid` – Full or abbreviated SHA of the commit to revert.
///
/// # Returns
/// The stdout of `git revert` on success, or stderr as an error.
#[tauri::command]
pub async fn revert_commit(oid: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.revert_commit(&oid).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Cherry-pick a commit onto the current branch.
///
/// # Arguments
/// - `oid` – Full or abbreviated SHA of the commit to cherry-pick.
///
/// # Returns
/// The stdout of `git cherry-pick` on success, or stderr as an error.
#[tauri::command]
pub async fn cherry_pick(oid: String, state: State<'_, AppState>) -> Result<String, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        let result = repo.cherry_pick(&oid).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Reset HEAD to a specific commit.
///
/// # Arguments
/// - `oid`  – Full or abbreviated SHA of the target commit.
/// - `mode` – Reset mode: `"soft"`, `"mixed"`, or `"hard"`.
///
/// # Returns
/// `Ok(())` on success, or an error string if the mode is invalid or
/// `git reset` exits with a non-zero status.
#[tauri::command]
pub async fn reset_to_commit(
    oid: String,
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.reset_to_commit(&oid, &mode).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Amend the most recent commit with a new message.
///
/// Any currently staged changes are included in the amended commit,
/// mirroring `git commit --amend -m <message>`.
///
/// # Arguments
/// - `message` – The replacement commit message.
///
/// # Returns
/// `Ok(())` on success, or an error string if `git commit --amend` fails.
#[tauri::command]
pub async fn amend_commit(message: String, state: State<'_, AppState>) -> Result<(), String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.amend_commit(&message).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Return the commit message of the current HEAD commit.
///
/// Useful for pre-filling an amend dialog with the existing message.
///
/// # Returns
/// The raw commit message string, or an error string if HEAD cannot be
/// resolved.
#[tauri::command]
pub fn get_head_message(state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        repo.get_head_message().map_err(|e| e.to_string())
    })
}

/// Push the current working-tree changes onto the stash stack.
///
/// # Parameters
/// - `message` – Optional stash description (equivalent to `git stash push -m <msg>`).
///
/// # Returns
/// The stdout of `git stash push` on success, or stderr as an error.
#[tauri::command]
pub fn stash_push(message: Option<String>, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        let result = repo
            .stash_push(message.as_deref())
            .map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
}

/// Pop (apply and drop) a stash entry.
///
/// # Parameters
/// - `index` – Zero-based stash index to pop (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash pop` on success, or stderr as an error.
#[tauri::command]
pub fn stash_pop(index: Option<usize>, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        let result = repo.stash_pop(index).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
}

/// Return a list of stash entry descriptions (one per stash entry).
///
/// Each string corresponds to a line from `git stash list`.
#[tauri::command]
pub fn stash_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    with_active_repo(&state, |repo| repo.stash_list().map_err(|e| e.to_string()))
}

/// Apply a stash entry without removing it.
///
/// # Parameters
/// - `index` – Zero-based stash index to apply (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash apply` on success, or stderr as an error.
#[tauri::command]
pub fn stash_apply(index: Option<usize>, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        let result = repo.stash_apply(index).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
}

/// Restore a single file from a stash entry into the working directory.
///
/// # Parameters
/// - `index` – Zero-based stash index.
/// - `path` – Repo-relative file path to restore.
///
/// # Returns
/// The stdout of `git restore` on success, or stderr as an error.
#[tauri::command]
pub fn stash_apply_file(
    index: usize,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        let result = repo
            .stash_apply_file(index, &path)
            .map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
}

/// Drop a stash entry without applying it.
///
/// # Parameters
/// - `index` – Zero-based stash index to drop (defaults to 0, i.e. the latest stash).
///
/// # Returns
/// The stdout of `git stash drop` on success, or stderr as an error.
#[tauri::command]
pub fn stash_drop(index: Option<usize>, state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        let result = repo.stash_drop(index).map_err(|e| e.to_string())?;
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
}

/// Return structured stash entries with parsed metadata.
///
/// Each entry includes index, message, branch, timestamp, and commit OID.
#[tauri::command]
pub fn stash_entries(state: State<'_, AppState>) -> Result<Vec<git_engine::StashEntry>, String> {
    with_active_repo(&state, |repo| {
        repo.stash_entries().map_err(|e| e.to_string())
    })
}

/// Return the diff of a stash entry as structured `FileDiff` objects.
///
/// # Parameters
/// - `index` – Zero-based stash index (defaults to 0, i.e. the latest stash).
#[tauri::command]
pub fn stash_show_parsed(
    index: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::FileDiff>, String> {
    with_active_repo(&state, |repo| {
        repo.stash_show_parsed(index).map_err(|e| e.to_string())
    })
}

// ---------------------------------------------------------------------------
// Multi-project tab commands
// ---------------------------------------------------------------------------

/// Open a repo as a new tab with lightweight metadata only.
///
/// If the path is already open, returns its existing slot info without duplicating.
/// Does NOT fully load the repo (no graph, no watcher). Call [`switch_project`] to activate.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the repository root.
///
/// # Returns
/// [`ProjectInfo`] with lightweight metadata on success, or an error string if the path
/// is not a valid git repository.
#[tauri::command]
pub fn open_project(path: String, state: State<'_, AppState>) -> Result<ProjectInfo, String> {
    let mut projects = state.projects.lock().map_err(|e| e.to_string())?;

    // Check if already open
    if let Some(existing) = projects.iter().find(|p| p.path == path) {
        return Ok(ProjectInfo {
            path: existing.path.clone(),
            name: existing.name.clone(),
            head_branch: existing.head_branch.clone(),
            change_count: existing.change_count,
        });
    }

    // Read lightweight metadata without building the graph
    let repo_path = PathBuf::from(&path);
    let temp_repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
    let status = temp_repo.status().map_err(|e| e.to_string())?;
    let change_count = temp_repo.file_statuses().map(|s| s.len()).unwrap_or(0);

    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());

    let slot = ProjectSlot {
        path: path.clone(),
        name: name.clone(),
        repo: None,
        layout: None,
        watcher: None,
        head_branch: status.head_branch.clone(),
        change_count,
    };

    projects.push(slot);

    // Persist to config
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        if !config.open_projects.contains(&path) {
            config.open_projects.push(path.clone());
        }
        config.recent_repos.retain(|r| r != &path);
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    Ok(ProjectInfo {
        path,
        name,
        head_branch: status.head_branch,
        change_count,
    })
}

/// Close a tab and remove it from the persisted list.
///
/// Adds the closed path to `recent_repos` (front, capped at 20). Adjusts the
/// active index if the closed tab was active or preceded the active tab.
///
/// # Parameters
/// - `index` – Zero-based index of the tab to close.
#[tauri::command]
pub fn close_project(index: usize, state: State<'_, AppState>) -> Result<(), String> {
    let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
    let mut active = state.active_index.lock().map_err(|e| e.to_string())?;

    if index >= projects.len() {
        return Err("Project index out of bounds".to_string());
    }

    let closed_path = projects[index].path.clone();
    projects.remove(index);

    // Adjust active index
    if projects.is_empty() {
        *active = None;
    } else if let Some(current) = *active {
        if current == index {
            *active = Some(index.min(projects.len() - 1));
        } else if current > index {
            *active = Some(current - 1);
        }
    }

    // Persist to config
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.open_projects.retain(|p| p != &closed_path);
        config.active_project_index = *active;
        config.recent_repos.retain(|r| r != &closed_path);
        config.recent_repos.insert(0, closed_path);
        config.recent_repos.truncate(20);
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Switch the active tab. Unloads the previous project's heavy data,
/// loads the target project fully (repo, graph, watcher).
///
/// # Parameters
/// - `index` – Zero-based index of the tab to switch to.
///
/// # Returns
/// [`RepoInfo`] for the newly active repo on success.
#[tauri::command]
pub async fn switch_project(
    index: usize,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<RepoInfo, String> {
    // 1. Unload the previous active project's heavy data
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        if let Some(prev_idx) = *active
            && let Some(prev_slot) = projects.get_mut(prev_idx)
        {
            if let Some(repo) = &prev_slot.repo {
                if let Ok(status) = repo.status() {
                    prev_slot.head_branch = status.head_branch;
                }
                prev_slot.change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);
            }
            prev_slot.repo = None;
            prev_slot.layout = None;
            prev_slot.watcher = None;
        }
    }

    // 2. Read the target path
    let path = {
        let projects = state.projects.lock().map_err(|e| e.to_string())?;
        let slot = projects
            .get(index)
            .ok_or_else(|| "Project index out of bounds".to_string())?;
        slot.path.clone()
    };

    // 3. Fully load the target project off-thread
    let path_clone = path.clone();
    let (repo, layout, status) = tokio::task::spawn_blocking(move || {
        let repo =
            git_engine::Repository::open(PathBuf::from(&path_clone)).map_err(|e| e.to_string())?;
        let commits = repo.walk_commits(50_000).map_err(|e| e.to_string())?;
        let graph_commits: Vec<GraphCommit> = commits
            .iter()
            .map(|c| GraphCommit {
                oid: c.oid.clone(),
                parents: c.parents.clone(),
                timestamp: c.timestamp,
                refs: c.refs.clone(),
                summary: c.summary.clone(),
                author: c.author.clone(),
                email: c.email.clone(),
            })
            .collect();
        let dag = Dag::build(&graph_commits);
        let layout = GraphLayout::compute(&dag);
        let status = repo.status().map_err(|e| e.to_string())?;
        Ok::<_, String>((repo, layout, status))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e: String| e)?;

    // 4. Start filesystem watcher
    let repo_path = PathBuf::from(&path);
    let handle = app_handle.clone();
    let new_watcher = watcher::RepoWatcher::start(&repo_path, move || {
        let _ = handle.emit("repo-changed", ());
    })
    .ok();

    let change_count = repo.file_statuses().map(|s| s.len()).unwrap_or(0);

    // 5. Store in slot and update active index
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        if let Some(slot) = projects.get_mut(index) {
            slot.repo = Some(repo);
            slot.layout = Some(layout);
            slot.watcher = new_watcher;
            slot.head_branch = status.head_branch.clone();
            slot.change_count = change_count;
        }
        let mut active = state.active_index.lock().map_err(|e| e.to_string())?;
        *active = Some(index);
    }

    // 6. Persist active index
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.active_project_index = Some(index);
        config.save(&state.config_path).map_err(|e| e.to_string())?;
    }

    // 7. Re-detect active provider for the new repo
    detect_active_provider(&state).await;

    Ok(RepoInfo {
        path: status.path,
        head_branch: status.head_branch,
        head_oid: status.head_oid,
        branch_count: status.branch_count,
    })
}

/// Return lightweight metadata for all open tabs.
#[tauri::command]
pub fn get_open_projects(state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    Ok(projects
        .iter()
        .map(|slot| ProjectInfo {
            path: slot.path.clone(),
            name: slot.name.clone(),
            head_branch: slot.head_branch.clone(),
            change_count: slot.change_count,
        })
        .collect())
}

/// Return the index of the currently active project.
#[tauri::command]
pub fn get_active_project_index(state: State<'_, AppState>) -> Result<Option<usize>, String> {
    Ok(*state.active_index.lock().map_err(|e| e.to_string())?)
}

/// Restore persisted project tabs from config on app startup.
///
/// Opens each path in `config.open_projects` as a lightweight slot (no graph).
/// Invalid paths (deleted repos) are silently skipped and removed from config.
/// If called multiple times, existing slots are cleared first to prevent duplicates.
///
/// # Returns
/// A [`Vec<ProjectInfo>`] of all successfully restored projects.
#[tauri::command]
pub fn restore_projects(state: State<'_, AppState>) -> Result<Vec<ProjectInfo>, String> {
    // Extract the paths from config then drop the lock immediately.
    let paths = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.open_projects.clone()
    };
    // config lock is dropped here.

    // Parallel phase: open repos and gather metadata (I/O-heavy, benefits from parallelism).
    // Invalid paths (deleted/moved repos) are silently dropped via `filter_map`.
    let results: Vec<(String, String, Option<String>, usize)> = paths
        .par_iter()
        .filter_map(|path| {
            let repo_path = PathBuf::from(path);
            let temp_repo = git_engine::Repository::open(repo_path).ok()?;
            let status = temp_repo.status().ok()?;
            let change_count = temp_repo.file_statuses().map(|s| s.len()).unwrap_or(0);
            let name = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            Some((path.clone(), name, status.head_branch, change_count))
        })
        .collect();

    let mut valid_paths = Vec::new();
    let mut infos = Vec::new();

    // Sequential phase: populate the shared projects vec (must hold the mutex).
    {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;

        // Clear existing slots to prevent duplicates on repeated calls.
        projects.clear();

        for (path, name, head_branch, change_count) in results {
            let slot = ProjectSlot {
                path: path.clone(),
                name: name.clone(),
                repo: None,
                layout: None,
                watcher: None,
                head_branch: head_branch.clone(),
                change_count,
            };

            projects.push(slot);
            valid_paths.push(path.clone());

            infos.push(ProjectInfo {
                path,
                name,
                head_branch,
                change_count,
            });
        }
    }
    // projects lock is dropped here before acquiring config again.

    // Update config to remove invalid paths.
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        config.open_projects = valid_paths;
        let _ = config.save(&state.config_path);
    }

    Ok(infos)
}

/// Return recent repos filtered to exclude already-open paths.
#[tauri::command]
pub fn get_recent_repos(state: State<'_, AppState>) -> Result<Vec<RecentRepo>, String> {
    // Acquire projects first, then config — consistent with the ordering used
    // elsewhere to avoid ABBA deadlocks.
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let config = state.config.lock().map_err(|e| e.to_string())?;
    let open_paths: Vec<&String> = projects.iter().map(|p| &p.path).collect();

    Ok(config
        .recent_repos
        .iter()
        .filter(|r| !open_paths.contains(r))
        .map(|r| {
            let name = std::path::Path::new(r)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| r.clone());
            RecentRepo {
                path: r.clone(),
                name,
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Remote operations
// ---------------------------------------------------------------------------

/// List all configured remotes for the active repository.
#[tauri::command]
pub fn get_remotes(state: State<'_, AppState>) -> Result<Vec<RemoteInfo>, String> {
    with_active_repo(&state, |repo| {
        let git_repo = repo.inner();
        let remotes = git_repo.remotes().map_err(|e| e.to_string())?;
        let mut result = Vec::new();
        for name in remotes.iter().flatten() {
            let url = git_repo
                .find_remote(name)
                .ok()
                .and_then(|r| r.url().map(|u| u.to_string()));
            result.push(RemoteInfo {
                name: name.to_string(),
                url,
            });
        }
        Ok(result)
    })
}

/// Get the filesystem path of the active project.
fn get_active_project_path(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let projects = state.projects.lock().map_err(|e| e.to_string())?;
    let active = state.active_index.lock().map_err(|e| e.to_string())?;
    let idx = active.ok_or_else(|| "No active project".to_string())?;
    let slot = projects
        .get(idx)
        .ok_or_else(|| "Active project index out of bounds".to_string())?;
    Ok(PathBuf::from(&slot.path))
}

/// Fetch all updates from a remote as a background task.
///
/// Spawns `git fetch <remote>` via the task manager and returns immediately
/// with the task ID. Progress streams to the task popover.
#[tauri::command]
pub async fn fetch_remote(
    remote: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Fetch {}", remote);
    let id = task_manager
        .spawn(label, "git", &["fetch", &remote], &cwd, true)
        .await;

    Ok(id)
}

/// Pull a branch from a remote (merge strategy) as a background task.
///
/// Spawns `git pull <remote> <branch>` via the task manager.
#[tauri::command]
pub async fn pull_remote(
    remote: String,
    branch: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Pull {}/{}", remote, branch);
    let id = task_manager
        .spawn(label, "git", &["pull", &remote, &branch], &cwd, true)
        .await;

    Ok(id)
}

/// Push a branch to a remote as a background task.
///
/// Spawns `git push <remote> <branch>` via the task manager.
#[tauri::command]
pub async fn push_remote(
    remote: String,
    branch: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;

    let label = format!("Push {}/{}", remote, branch);
    let id = task_manager
        .spawn(label, "git", &["push", &remote, &branch], &cwd, true)
        .await;

    Ok(id)
}

/// Renames a remote in the active repository.
///
/// Equivalent to `git remote rename <old_name> <new_name>`. Returns an error
/// if `old_name` does not exist or `new_name` is already taken.
#[tauri::command]
pub fn rename_remote(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.rename_remote(&old_name, &new_name)
            .map_err(|e| e.to_string())
    })
}

/// Removes a remote from the active repository.
///
/// Equivalent to `git remote remove <name>`. Returns an error if the remote
/// does not exist.
#[tauri::command]
pub fn remove_remote(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.remove_remote(&name).map_err(|e| e.to_string())
    })
}

// ---------------------------------------------------------------------------
// Tag commands
// ---------------------------------------------------------------------------

/// Return all tags in the active repository, sorted newest-version-first.
#[tauri::command]
pub fn list_tags(state: State<'_, AppState>) -> Result<Vec<git_engine::TagInfo>, String> {
    with_active_repo(&state, |repo| repo.tags().map_err(|e| e.to_string()))
}

/// Return diff statistics for a single commit.
#[tauri::command]
pub fn get_commit_stats(
    oid: String,
    state: State<'_, AppState>,
) -> Result<git_engine::CommitStats, String> {
    with_active_repo(&state, |repo| {
        repo.commit_stats(&oid).map_err(|e| e.to_string())
    })
}

/// List tags with pagination, sorted newest-version-first.
#[tauri::command]
pub fn list_tags_paginated(
    per_page: u32,
    page: u32,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::TagInfo>, String> {
    with_active_repo(&state, |repo| {
        repo.tags_paginated(per_page, page)
            .map_err(|e| e.to_string())
    })
}

/// Search all tags by name substring (case-insensitive).
#[tauri::command]
pub fn search_tags(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<git_engine::TagInfo>, String> {
    with_active_repo(&state, |repo| {
        repo.search_tags(&query).map_err(|e| e.to_string())
    })
}

/// Create a new tag in the active repository.
///
/// - If `message` is provided and non-empty, creates an annotated tag.
/// - Otherwise creates a lightweight tag.
/// - If `target` is empty, tags HEAD.
#[tauri::command]
pub fn create_tag(
    name: String,
    target: String,
    message: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        let msg = message.as_deref().filter(|m| !m.is_empty());
        let result = if target.is_empty() {
            repo.create_tag(&name, msg).map_err(|e| e.to_string())?
        } else {
            match msg {
                Some(m) => repo
                    .git_cmd(&["tag", "-a", &name, &target, "-m", m])
                    .map_err(|e| e.to_string())?,
                None => repo
                    .git_cmd(&["tag", &name, &target])
                    .map_err(|e| e.to_string())?,
            }
        };
        if result.success {
            Ok(())
        } else {
            Err(result.stderr)
        }
    })
}

/// Delete a local tag by name.
#[tauri::command]
pub fn delete_tag(name: String, state: State<'_, AppState>) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        let result = repo.delete_tag(&name).map_err(|e| e.to_string())?;
        if result.success {
            Ok(())
        } else {
            Err(result.stderr)
        }
    })
}

/// Push a tag to a remote as a background task.
#[tauri::command]
pub async fn push_tag(
    tag_name: Option<String>,
    remote: String,
    state: State<'_, AppState>,
    task_manager: State<'_, Arc<TaskManager>>,
) -> Result<TaskId, String> {
    let cwd = get_active_project_path(&state)?;
    let remote = if remote.is_empty() {
        "origin".to_string()
    } else {
        remote
    };
    match tag_name {
        Some(name) => {
            let label = format!("Push tag {}", name);
            let tag_ref = format!("refs/tags/{}", name);
            let id = task_manager
                .spawn(label, "git", &["push", &remote, &tag_ref], &cwd, true)
                .await;
            Ok(id)
        }
        None => {
            let label = "Push all tags".to_string();
            let id = task_manager
                .spawn(label, "git", &["push", &remote, "--tags"], &cwd, true)
                .await;
            Ok(id)
        }
    }
}

// ---------------------------------------------------------------------------
// Conflict commands
// ---------------------------------------------------------------------------

/// Return the current conflict state and list of conflicted file paths.
#[tauri::command]
pub fn get_conflict_status(
    state: State<'_, AppState>,
) -> Result<git_engine::ConflictStatus, String> {
    with_active_repo(&state, |repo| {
        repo.conflict_status().map_err(|e| e.to_string())
    })
}

/// Abort the current mid-operation git state (merge/rebase/cherry-pick/revert).
#[tauri::command]
pub fn abort_operation(state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        let conflict_state = repo.detect_conflict_state();
        let result = match conflict_state {
            git_engine::ConflictState::Merging => repo.abort_merge().map_err(|e| e.to_string())?,
            git_engine::ConflictState::Rebasing => {
                repo.abort_rebase().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::CherryPicking => {
                repo.abort_cherry_pick().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Reverting => {
                repo.abort_revert().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::None => {
                return Err("No operation in progress to abort".to_string());
            }
        };
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
}

/// Continue the current mid-operation git state after conflicts are resolved.
#[tauri::command]
pub fn continue_operation(state: State<'_, AppState>) -> Result<String, String> {
    with_active_repo(&state, |repo| {
        let status = repo.conflict_status().map_err(|e| e.to_string())?;
        if status.state == git_engine::ConflictState::None {
            return Err("No operation in progress to continue".to_string());
        }
        if !status.can_continue {
            return Err("Cannot continue: unresolved conflicts remain".to_string());
        }
        let result = match status.state {
            git_engine::ConflictState::Merging => {
                repo.continue_merge().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Rebasing => {
                repo.continue_rebase().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::CherryPicking => {
                repo.continue_cherry_pick().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::Reverting => {
                repo.continue_revert().map_err(|e| e.to_string())?
            }
            git_engine::ConflictState::None => unreachable!(),
        };
        if result.success {
            Ok(result.stdout)
        } else {
            Err(result.stderr)
        }
    })
}

// ---------------------------------------------------------------------------
// Provider auth commands
// ---------------------------------------------------------------------------

/// Connect to a git hosting provider using a Personal Access Token (PAT).
///
/// Validates the token, stores it in the encrypted credential store,
/// builds a [`ProviderConnection`][crate::state::ProviderConnection] with
/// the authenticated user's profile, and appends it to the providers vec
/// (or replaces an existing entry with the same `instance_url`).
///
/// After connecting, re-runs active provider detection against the current
/// repo's remote URL and persists all providers to `settings.json`.
///
/// # Parameters
/// - `kind`         – Provider type (`"gitlab"` or `"github"`).
/// - `instance_url` – Base URL (e.g. `"https://gitlab.com"` or `"https://api.github.com"`).
/// - `token`        – Personal Access Token.
///
/// # Returns
/// The authenticated user profile as a [`provider::ProviderUser`].
#[tauri::command]
pub async fn connect_provider(
    kind: provider::ProviderKind,
    instance_url: String,
    token: String,
    state: State<'_, AppState>,
) -> Result<provider::ProviderUser, String> {
    // 1. Validate token
    let user = match kind {
        provider::ProviderKind::GitLab => auth::validate_gitlab_pat(&instance_url, &token).await,
        provider::ProviderKind::GitHub => auth::validate_github_pat(&instance_url, &token).await,
    }
    .map_err(|e| e.to_string())?;

    // 2. Store credential
    let credential = auth::Credential {
        token: token.clone(),
        provider: kind,
    };
    state
        .credential_store
        .store_credential(&instance_url, &credential)
        .map_err(|e| e.to_string())?;

    // 3. Build ProviderConnection (metadata only, no CiProvider)
    let conn = crate::state::ProviderConnection {
        kind,
        instance_url: instance_url.clone(),
        user: user.clone(),
        project_ref: None,
        project_name: None,
    };

    // 4. Insert or replace in providers vec
    {
        let mut providers = state.providers.lock().unwrap();
        if let Some(pos) = providers
            .iter()
            .position(|p| p.instance_url == instance_url)
        {
            providers[pos] = conn;
        } else {
            providers.push(conn);
        }
    }

    // 5. Save to config and detect active provider
    save_providers_to_config(&state);
    detect_active_provider(&state).await;

    Ok(user)
}

/// Disconnect a specific provider identified by its instance URL.
///
/// Removes the provider from the in-memory vec, deletes the credential
/// from the encrypted store, saves the updated config, and re-runs
/// active provider detection.
///
/// # Parameters
/// - `instance_url` – Base URL of the provider to disconnect.
#[tauri::command]
pub async fn disconnect_provider(
    instance_url: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Remove from providers vec
    {
        let mut providers = state.providers.lock().unwrap();
        providers.retain(|p| p.instance_url != instance_url);
    }

    // Delete credential
    let _ = state.credential_store.delete_credential(&instance_url);

    // Save config and re-detect
    save_providers_to_config(&state);
    detect_active_provider(&state).await;

    Ok(())
}

/// Attempt to restore all previously saved provider sessions on app startup.
///
/// Reads the `providers` list from `settings.json`, retrieves each token from
/// the credential store, validates it against the provider API, and builds
/// a [`ProviderConnection`][crate::state::ProviderConnection] for each
/// successful validation.
///
/// After reconnecting, runs active provider detection against the current
/// repo's remote URL.
///
/// # Returns
/// A list of successfully reconnected user profiles.
#[tauri::command]
pub async fn try_auto_connect(
    state: State<'_, AppState>,
) -> Result<Vec<provider::ProviderUser>, String> {
    // Read saved providers from config
    let saved_providers = {
        let config = state.config.lock().unwrap();
        config.providers.clone()
    };

    let mut connected_users = Vec::new();
    let mut connections = Vec::new();

    for saved in &saved_providers {
        let kind = match provider::ProviderKind::from_config_str(&saved.kind) {
            Some(k) => k,
            None => continue,
        };

        // Get token from credential store
        let credential = match state.credential_store.get_credential(&saved.instance_url) {
            Ok(Some(c)) => c,
            _ => continue,
        };

        // Validate token
        let user = match kind {
            provider::ProviderKind::GitLab => {
                auth::validate_gitlab_pat(&saved.instance_url, &credential.token).await
            }
            provider::ProviderKind::GitHub => {
                auth::validate_github_pat(&saved.instance_url, &credential.token).await
            }
        };

        let user = match user {
            Ok(u) => u,
            Err(_) => continue,
        };

        connections.push(crate::state::ProviderConnection {
            kind,
            instance_url: saved.instance_url.clone(),
            user: user.clone(),
            project_ref: None,
            project_name: None,
        });
        connected_users.push(user);
    }

    // Store all successful connections
    *state.providers.lock().unwrap() = connections;

    // Detect active provider from repo remote
    detect_active_provider(&state).await;

    Ok(connected_users)
}

/// Return the current multi-provider connection status.
///
/// Builds a [`provider::ProviderStatusResponse`] containing all authenticated
/// providers and which one (if any) is active for the currently open repository.
/// Used by the frontend to render the provider list and active badge.
#[tauri::command]
pub fn get_provider_status(state: State<'_, AppState>) -> provider::ProviderStatusResponse {
    let providers = state.providers.lock().unwrap();
    let active_index = *state.active_provider_index.lock().unwrap();

    let connected: Vec<provider::ConnectedProvider> = providers
        .iter()
        .map(|p| provider::ConnectedProvider {
            kind: p.kind.as_str().to_string(),
            instance_url: p.instance_url.clone(),
            user: p.user.clone(),
            project_name: p.project_name.clone(),
        })
        .collect();

    provider::ProviderStatusResponse {
        providers: connected,
        active_index,
    }
}

// ---------------------------------------------------------------------------
// CI run commands
// ---------------------------------------------------------------------------

/// Fetch a paginated list of CI runs for the detected project.
///
/// All filter parameters are forwarded to the provider. Filtering is performed
/// server-side only — there is no client-side filtering.
#[tauri::command]
pub async fn list_ci_runs(
    branch: Option<String>,
    source: Option<String>,
    status: Option<String>,
    per_page: Option<u32>,
    page: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<provider::CiRun>, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    let filters = provider::CiFilters {
        branch,
        status,
        source,
    };
    ci_provider
        .list_ci_runs(
            &project_ref,
            &filters,
            per_page.unwrap_or(20),
            page.unwrap_or(1),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Fetch full detail for a single CI run, including its stages and jobs.
#[tauri::command]
pub async fn get_ci_run_detail(
    run_id: u64,
    state: State<'_, AppState>,
) -> Result<provider::CiRunDetail, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .get_ci_run_detail(&project_ref, run_id)
        .await
        .map_err(|e| e.to_string())
}

/// Fetch the raw log output for a single CI job.
#[tauri::command]
pub async fn get_job_log(job_id: u64, state: State<'_, AppState>) -> Result<String, String> {
    let (ci_provider, project_ref) = get_active_provider_and_project(&state)?;
    ci_provider
        .get_job_log(&project_ref, job_id)
        .await
        .map_err(|e| e.to_string())
}

/// Preprocess a raw CI job log, stripping provider-specific noise.
///
/// Delegates to [`provider::log_preprocessor::preprocess_ci_log`] which strips
/// timestamps, stream codes, section markers, and adds line numbers. ANSI
/// color/style codes are preserved for the frontend renderer.
#[tauri::command]
pub fn preprocess_job_log(raw_text: String, provider_kind: String) -> Result<String, String> {
    let kind = match provider_kind.as_str() {
        "gitlab" => provider::ProviderKind::GitLab,
        "github" => provider::ProviderKind::GitHub,
        _ => return Err(format!("Unknown provider kind: {}", provider_kind)),
    };
    Ok(provider::log_preprocessor::preprocess_ci_log(
        &raw_text, kind,
    ))
}

/// Re-detect the active provider from the currently open repository's remote URL.
///
/// Iterates all connected providers, matches the remote URL against each,
/// and sets the active provider index on the first match. Clears project info
/// on all non-matching providers.
///
/// Call this after opening a new repo when providers are already connected,
/// so the CI panel automatically scopes to the correct project.
#[tauri::command]
pub async fn detect_project(state: State<'_, AppState>) -> Result<(), String> {
    detect_active_provider(&state).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

/// Return the persisted UI locale tag (e.g. `"en-US"`).
#[tauri::command]
pub fn get_locale(state: State<'_, AppState>) -> String {
    let config = state.config.lock().unwrap();
    config.locale.clone()
}

/// Change the persisted UI locale tag.
#[tauri::command]
pub fn set_locale(locale: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.locale = locale;
    config.save(&state.config_path).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// User identity
// ---------------------------------------------------------------------------

/// Return the current user's identities (emails and names) for author highlighting.
///
/// Collects `user.email` and `user.name` from git config plus any connected
/// provider user emails, display names, and usernames. Returns a deduplicated,
/// lowercased list of all identity strings.
#[tauri::command]
pub fn get_user_identities(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let mut identities: Vec<String> = Vec::new();

    // Git config email and name from active repo
    if let Ok(email) = with_active_repo(&state, |repo| {
        let config = repo.inner().config().map_err(|e| e.to_string())?;
        config.get_string("user.email").map_err(|e| e.to_string())
    }) {
        let lower = email.to_lowercase();
        if !lower.is_empty() {
            identities.push(lower);
        }
    }
    if let Ok(name) = with_active_repo(&state, |repo| {
        let config = repo.inner().config().map_err(|e| e.to_string())?;
        config.get_string("user.name").map_err(|e| e.to_string())
    }) {
        let lower = name.to_lowercase();
        if !lower.is_empty() {
            identities.push(lower);
        }
    }

    // Connected provider identities (email, display_name, username)
    if let Ok(providers) = state.providers.lock() {
        for conn in providers.iter() {
            if let Some(ref email) = conn.user.email {
                let lower = email.to_lowercase();
                if !lower.is_empty() {
                    identities.push(lower);
                }
            }
            let display = conn.user.display_name.to_lowercase();
            if !display.is_empty() {
                identities.push(display);
            }
            let username = conn.user.username.to_lowercase();
            if !username.is_empty() {
                identities.push(username);
            }
        }
    }

    identities.sort();
    identities.dedup();
    Ok(identities)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a `Box<dyn CiProvider>` from provider metadata and a token.
///
/// Centralizes the provider construction logic to avoid repeating the
/// match on `ProviderKind` throughout the codebase.
fn create_ci_provider(
    kind: provider::ProviderKind,
    base_url: &str,
    token: &str,
) -> Box<dyn provider::CiProvider> {
    match kind {
        provider::ProviderKind::GitLab => {
            Box::new(gitlab_api::GitLabProvider::new(base_url, token))
        }
        provider::ProviderKind::GitHub => {
            Box::new(github_api::GitHubProvider::new(base_url, token))
        }
    }
}

/// Extract the active provider's CI client and project reference from state.
///
/// Reads `active_provider_index` to find the active
/// [`ProviderConnection`][crate::state::ProviderConnection], retrieves its
/// token from the credential store, and creates a fresh `Box<dyn CiProvider>`.
///
/// Returns an error if no provider is active or no project is detected.
fn get_active_provider_and_project(
    state: &State<'_, AppState>,
) -> Result<(Box<dyn provider::CiProvider>, String), String> {
    let (kind, base_url, project_ref) = {
        let providers = state.providers.lock().unwrap();
        let active_idx = state.active_provider_index.lock().unwrap();
        let idx = active_idx.ok_or("No active provider")?;
        let conn = providers
            .get(idx)
            .ok_or("Active provider index out of bounds")?;
        let project_ref = conn.project_ref.clone().ok_or("No project detected")?;
        (conn.kind, conn.instance_url.clone(), project_ref)
    };

    let credential = state
        .credential_store
        .get_credential(&base_url)
        .map_err(|e| e.to_string())?
        .ok_or("No credential found for active provider")?;

    let ci_provider = create_ci_provider(kind, &base_url, &credential.token);
    Ok((ci_provider, project_ref))
}

/// Detect which provider (if any) matches the current repo's remote URL
/// and set it as the active provider.
///
/// Iterates all entries in the providers vec, calls
/// [`provider::parse_remote_url`] against each, and on the first match
/// verifies the project via the provider API. Sets `active_provider_index`
/// to the matching entry and stores `project_ref` / `project_name` on it.
/// Clears project info on all non-matching providers.
///
/// If no repo is open or no provider matches, `active_provider_index` is
/// set to `None`.
async fn detect_active_provider(state: &State<'_, AppState>) {
    // Get the repo's origin remote URL from the active slot
    let remote_url = {
        let projects = state.projects.lock().unwrap();
        let active = state.active_index.lock().unwrap();
        active
            .and_then(|idx| projects.get(idx))
            .and_then(|slot| slot.repo.as_ref())
            .and_then(extract_origin_url)
    };

    let remote_url = match remote_url {
        Some(url) => url,
        None => {
            // No repo open — clear active index and all project info
            *state.active_provider_index.lock().unwrap() = None;
            let mut providers = state.providers.lock().unwrap();
            for p in providers.iter_mut() {
                p.project_ref = None;
                p.project_name = None;
            }
            return;
        }
    };

    // Snapshot provider metadata (kind, url) so we don't hold the lock across await
    let provider_snapshots: Vec<(usize, provider::ProviderKind, String)> = {
        let providers = state.providers.lock().unwrap();
        providers
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.kind, p.instance_url.clone()))
            .collect()
    };

    let mut matched_index: Option<usize> = None;
    let mut matched_project_ref: Option<String> = None;
    let mut matched_project_name: Option<String> = None;

    for (idx, kind, instance_url) in &provider_snapshots {
        let parsed =
            provider::parse_remote_url(&remote_url, Some(instance_url.as_str()), Some(*kind));

        let project_ref = match parsed {
            Some((_, ref_)) => ref_,
            None => continue,
        };

        // Get token to verify project
        let credential = match state.credential_store.get_credential(instance_url) {
            Ok(Some(c)) => c,
            _ => continue,
        };

        let ci_provider = create_ci_provider(*kind, instance_url, &credential.token);

        // Verify the project exists via the API
        match ci_provider.get_project(&project_ref).await {
            Ok(project) => {
                matched_index = Some(*idx);
                matched_project_ref = Some(project_ref);
                matched_project_name = Some(project.full_path);
                break; // First match wins
            }
            Err(_) => continue,
        }
    }

    // Update providers vec with match results
    {
        let mut providers = state.providers.lock().unwrap();
        for (i, p) in providers.iter_mut().enumerate() {
            if Some(i) == matched_index {
                p.project_ref = matched_project_ref.clone();
                p.project_name = matched_project_name.clone();
            } else {
                p.project_ref = None;
                p.project_name = None;
            }
        }
    }

    *state.active_provider_index.lock().unwrap() = matched_index;
}

/// Persist the current providers vec to `settings.json`.
///
/// Builds a `Vec<SavedProvider>` from the in-memory provider connections
/// and writes it to the config file.
fn save_providers_to_config(state: &State<'_, AppState>) {
    let saved: Vec<storage::config::SavedProvider> = {
        let providers = state.providers.lock().unwrap();
        providers
            .iter()
            .map(|p| storage::config::SavedProvider {
                kind: p.kind.as_str().to_string(),
                instance_url: p.instance_url.clone(),
            })
            .collect()
    };

    let mut config = state.config.lock().unwrap();
    config.providers = saved;
    let _ = config.save(&state.config_path);
}

// ---------------------------------------------------------------------------
// Theme commands
// ---------------------------------------------------------------------------

/// List all available themes (built-in + user-installed).
#[tauri::command]
pub fn list_themes(state: State<'_, AppState>) -> Vec<storage::ThemeMeta> {
    let _ = &state;
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let themes_dir = config_dir.join("themes");
    let _ = storage::theme::ensure_themes_dir(&themes_dir);
    storage::theme::list_all_themes(&themes_dir)
}

/// Resolve a full theme by name (built-in or user file).
#[tauri::command]
pub fn get_theme(name: String, state: State<'_, AppState>) -> storage::Theme {
    let _ = &state;
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let themes_dir = config_dir.join("themes");
    storage::theme::resolve_theme(&name, &themes_dir)
}

/// Set the active theme name and emit a `theme-changed` event with the resolved theme.
#[tauri::command]
pub fn set_theme(name: String, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    let mut config = storage::AppConfig::load(&config_path).unwrap_or_default();
    config.theme = name.clone();
    config.save(&config_path).map_err(|e| e.to_string())?;

    // Also update the in-memory config
    let mut cfg = state.config.lock().unwrap();
    cfg.theme = name.clone();

    let themes_dir = config_dir.join("themes");
    let theme = storage::theme::resolve_theme(&name, &themes_dir);
    let _ = app.emit("theme-changed", &theme);
    Ok(())
}

/// Get the current `theme_auto` setting.
#[tauri::command]
pub fn get_theme_auto(_state: State<'_, AppState>) -> bool {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    storage::AppConfig::load(&config_path)
        .map(|c| c.theme_auto)
        .unwrap_or(true)
}

/// Set the `theme_auto` preference and persist to config.
#[tauri::command]
pub fn set_theme_auto(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    let mut config = storage::AppConfig::load(&config_path).unwrap_or_default();
    config.theme_auto = enabled;
    config.save(&config_path).map_err(|e| e.to_string())?;

    // Also update the in-memory config
    let mut cfg = state.config.lock().unwrap();
    cfg.theme_auto = enabled;

    Ok(())
}

/// Resolve the startup theme, respecting the `theme_auto` setting and OS dark/light mode.
#[tauri::command]
pub fn resolve_startup_theme(app: AppHandle, _state: State<'_, AppState>) -> storage::Theme {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("beardgit");
    let config_path = config_dir.join("settings.json");
    let themes_dir = config_dir.join("themes");
    let _ = storage::theme::ensure_themes_dir(&themes_dir);

    let config = storage::AppConfig::load(&config_path).unwrap_or_default();

    let theme_id = if config.theme_auto {
        use tauri::Manager as _;
        let os_dark = app
            .get_webview_window("main")
            .and_then(|w| w.theme().ok())
            .map(|t| matches!(t, tauri::Theme::Dark))
            .unwrap_or(true);

        resolve_theme_for_mode(&config.theme, os_dark)
    } else {
        config.theme.clone()
    };

    storage::theme::resolve_theme(&theme_id, &themes_dir)
}

/// Given a base theme id and whether the OS is in dark mode, resolve the correct variant.
///
/// E.g. `"github-dark"` + light mode → `"github-light"`.
pub fn resolve_theme_for_mode(base: &str, os_dark: bool) -> String {
    if os_dark {
        if base.ends_with("-light") {
            base.replace("-light", "-dark")
        } else {
            base.to_string()
        }
    } else if base.ends_with("-dark") {
        base.replace("-dark", "-light")
    } else {
        base.to_string()
    }
}

// ---------------------------------------------------------------------------
// Worktrees
// ---------------------------------------------------------------------------

/// List all worktrees for the active repository, including the main worktree.
///
/// Returns a [`WorktreeInfo`] for each worktree. The first element is always
/// the main worktree.
#[tauri::command]
pub fn list_worktrees(state: State<'_, AppState>) -> Result<Vec<git_engine::WorktreeInfo>, String> {
    with_active_repo(&state, |repo| {
        repo.list_worktrees().map_err(|e| e.to_string())
    })
}

/// Create a new linked worktree at `path` on `branch`.
///
/// # Parameters
/// - `path` – Absolute filesystem path where the new worktree directory will be created.
/// - `branch` – Branch name to check out (or create when `create_branch` is `true`).
/// - `create_branch` – When `true`, create a new branch with `-b`; when `false`, check
///   out an existing branch.
#[tauri::command]
pub fn create_worktree(
    path: String,
    branch: String,
    create_branch: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.create_worktree(&path, &branch, create_branch)
            .map_err(|e| e.to_string())
    })
}

/// Remove a linked worktree at `path`.
///
/// # Parameters
/// - `path` – Absolute filesystem path to the worktree directory to remove.
/// - `force` – When `true`, remove the worktree even if it has uncommitted changes
///   or is locked.
#[tauri::command]
pub fn remove_worktree(
    path: String,
    force: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    with_active_repo(&state, |repo| {
        repo.remove_worktree(&path, force)
            .map_err(|e| e.to_string())
    })
}

/// Extract the origin remote URL from a repository (synchronous, no await).
fn extract_origin_url(repo: &git_engine::Repository) -> Option<String> {
    let git_repo = repo.inner();
    let remote = git_repo.find_remote("origin").ok()?;
    let url = remote.url()?.to_string();
    Some(url)
}
