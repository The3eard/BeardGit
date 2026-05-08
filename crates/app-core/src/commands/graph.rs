//! Graph viewport, commit search, and commit detail commands.

use std::path::PathBuf;

use graph_builder::{Dag, GraphCommit, GraphLayout};
use tauri::State;
use tracing::instrument;

use super::graph_cache::load_or_build_layout;
use super::helpers::*;
use crate::state::AppState;

/// Return a paginated slice of the commit graph for virtual-scroll rendering.
///
/// # Parameters
/// - `offset` – Zero-based row index of the first commit to include.
/// - `limit`  – Maximum number of rows to return.
///
/// # Returns
/// A [`GraphViewport`] containing the layout nodes for the requested window.
#[tauri::command]
#[instrument(skip(state), name = "cmd::graph::viewport")]
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
        // This command returns a slice of the fully-loaded cached layout,
        // so "more" is a meaningless concept — the entire graph is already
        // in memory. Paginated streaming lives in `load_graph_chunk`.
        has_more: false,
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
                0,
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

        let total_commits = graph_commits.len();
        let dag = Dag::build(graph_commits);
        let layout = GraphLayout::compute(dag);
        let vp = layout.viewport(0, total_commits);

        Ok(GraphViewport {
            nodes: vp.nodes,
            lane_segments: vp.lane_segments,
            merge_curves: vp.merge_curves,
            total_count: vp.total_count,
            offset: 0,
            visible_lane_count: vp.visible_lane_count,
            total_lane_count: layout.lane_count,
            head_lane: vp.head_lane,
            // Search returns the full matched set in one shot, bounded by
            // `max_count`. Pagination is not wired through here today.
            has_more: false,
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

/// Return diff statistics for a single commit.
#[tauri::command]
pub async fn get_commit_stats(
    oid: String,
    state: State<'_, AppState>,
) -> Result<git_engine::CommitStats, String> {
    let repo_path = get_active_project_path(&state)?;
    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        repo.commit_stats(&oid).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Core chunk-building logic, separated from Tauri state/async so it can be
/// exercised in unit tests against a `git_engine::Repository` fixture.
///
/// Walks `limit + 1` commits starting at `offset` to probe whether more
/// commits exist beyond the window (used to populate
/// [`GraphViewport::has_more`]), then builds a per-chunk DAG and layout over
/// exactly `limit` truncated commits.
///
/// # Parameters
/// - `repo`   – Repository to walk.
/// - `offset` – Zero-based index of the first commit to return.
/// - `limit`  – Maximum number of commits to include in the chunk.
fn build_graph_chunk(
    repo: &git_engine::Repository,
    offset: usize,
    limit: usize,
) -> Result<GraphViewport, String> {
    // Walk one extra so we can detect whether more commits exist without
    // paying for a second round-trip.
    let commits = repo
        .walk_commits(offset, limit + 1)
        .map_err(|e| e.to_string())?;

    let has_more = commits.len() > limit;
    let truncated: Vec<_> = commits.into_iter().take(limit).collect();

    let graph_commits: Vec<GraphCommit> = truncated
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

    let total_commits = graph_commits.len();
    let dag = Dag::build(graph_commits);
    let layout = GraphLayout::compute(dag);
    let vp = layout.viewport(0, total_commits);

    Ok(GraphViewport {
        nodes: vp.nodes,
        lane_segments: vp.lane_segments,
        merge_curves: vp.merge_curves,
        total_count: vp.total_count,
        // Report the caller-requested starting index, not the inner viewport's
        // offset (which is always 0 because we build a fresh per-chunk layout).
        offset,
        visible_lane_count: vp.visible_lane_count,
        total_lane_count: layout.lane_count,
        head_lane: vp.head_lane,
        has_more,
    })
}

/// Load a paginated chunk of commits from the active repository, build a
/// per-chunk graph layout, and return it as a [`GraphViewport`].
///
/// Unlike [`get_graph_viewport`] (which slices a pre-computed cached layout),
/// this command opens the repo and walks commits on demand. Used by the
/// frontend to stream the graph in fixed-size windows as the user scrolls.
///
/// # Parameters
/// - `offset` – Zero-based index of the first commit to return (skips this
///   many entries from the topological revwalk).
/// - `limit`  – Maximum number of commits to include in the returned chunk.
///
/// # Returns
/// A [`GraphViewport`] whose `has_more` is `true` when additional commits
/// exist beyond the chunk.
#[tauri::command]
#[instrument(skip(state), name = "cmd::graph::load_chunk")]
pub async fn load_graph_chunk(
    offset: usize,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<GraphViewport, String> {
    let repo_path = get_active_project_path(&state)?;

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        build_graph_chunk(&repo, offset, limit)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Core of [`refresh_graph_layout`]: open the repo at `path`, consult
/// the persistent cache, and return the freshly-loaded layout.
///
/// Separated from the Tauri command so unit tests can exercise the
/// rebuild behaviour without a live `AppState`. On a cache hit the
/// returned layout is byte-identical to the previous build for the same
/// `(path, HEAD oid, refs)`; on a miss [`load_or_build_layout`] walks
/// the repo, computes a fresh layout, and writes the cache entry back.
fn rebuild_layout_blocking(
    path: &str,
    config_dir: &std::path::Path,
) -> Result<GraphLayout, String> {
    let repo = git_engine::Repository::open(PathBuf::from(path)).map_err(|e| e.to_string())?;
    let (layout, _was_cached) = load_or_build_layout(&repo, path, config_dir)?;
    Ok(layout)
}

/// Rebuild the active project's cached [`GraphLayout`] from the current
/// repository state.
///
/// [`get_graph_viewport`] slices a pre-computed layout that lives in
/// [`crate::state::ProjectSlot::layout`] and is only populated by
/// [`super::repository::open_repo`] and [`super::project::switch_project`].
/// After a mutation that changes reachable commits or refs (commit, amend,
/// push, pull, rebase, reset, …) the slot layout goes stale and the
/// viewport no longer reflects reality.
///
/// Frontend bridges invoke this command whenever they need the graph to
/// catch up — typically right after `commit()` or when a Fetch / Pull /
/// Push task reaches a completed lifecycle state. The work reuses
/// [`load_or_build_layout`] so the persistent on-disk cache correctly
/// misses on the new HEAD + refs key and writes a fresh entry in the
/// background.
///
/// # Returns
/// `Ok(())` when the layout was rebuilt, or an error string when no
/// project is active / no repository is loaded in the active slot.
#[tauri::command]
#[instrument(skip(state), name = "cmd::graph::refresh_layout")]
pub async fn refresh_graph_layout(state: State<'_, AppState>) -> Result<(), String> {
    // Snapshot the path + config dir so we can drop the lock before doing
    // the (potentially expensive) walk + layout build off-thread.
    let (path, config_dir) = {
        let projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        let idx = active.ok_or_else(|| "No active project".to_string())?;
        let slot = projects
            .get(idx)
            .ok_or_else(|| "Active project index out of bounds".to_string())?;
        (slot.path.clone(), state.config_dir.clone())
    };

    let path_clone = path.clone();
    let layout =
        tokio::task::spawn_blocking(move || rebuild_layout_blocking(&path_clone, &config_dir))
            .await
            .map_err(|e| e.to_string())??;

    // Re-acquire the lock and swap in the fresh layout. Only touch the slot
    // whose path still matches what we snapshotted — a project switch that
    // raced with this call is a silent no-op.
    let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
    let active = state.active_index.lock().map_err(|e| e.to_string())?;
    if let Some(idx) = *active
        && let Some(slot) = projects.get_mut(idx)
        && slot.path == path
    {
        slot.layout = Some(layout);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git_engine::test_support::create_repo_with_n_commits;

    #[test]
    fn build_graph_chunk_returns_offset_slice() {
        let (_dir, path) = create_repo_with_n_commits(50);
        let repo = git_engine::Repository::open(&path).unwrap();

        let chunk = build_graph_chunk(&repo, 10, 20).expect("chunk ok");
        assert_eq!(chunk.nodes.len(), 20);
        assert_eq!(chunk.offset, 10);
        assert!(
            chunk.has_more,
            "offset 10 + limit 20 of 50 commits should have more"
        );
    }

    #[test]
    fn build_graph_chunk_flags_last_page() {
        let (_dir, path) = create_repo_with_n_commits(50);
        let repo = git_engine::Repository::open(&path).unwrap();

        let chunk = build_graph_chunk(&repo, 40, 20).expect("chunk ok");
        assert_eq!(chunk.nodes.len(), 10);
        assert!(
            !chunk.has_more,
            "last 10 commits should be marked as final page"
        );
    }

    #[test]
    fn build_graph_chunk_offset_beyond_total_returns_empty() {
        let (_dir, path) = create_repo_with_n_commits(50);
        let repo = git_engine::Repository::open(&path).unwrap();

        let chunk = build_graph_chunk(&repo, 100, 20).expect("chunk ok");
        assert!(chunk.nodes.is_empty());
        assert!(!chunk.has_more);
    }

    /// After a new commit lands in the repo, `rebuild_layout_blocking`
    /// must return a layout that includes the fresh HEAD — not a cached
    /// layout from the previous HEAD. The persistent cache in
    /// `graph_cache.rs` keys on `(path, HEAD, refs)` so a HEAD move
    /// invalidates automatically; this test pins down that behaviour end
    /// to end through the refresh helper used by `refresh_graph_layout`.
    #[test]
    fn rebuild_layout_blocking_sees_new_commit_after_head_moves() {
        let (_dir, path) = create_repo_with_n_commits(5);
        let path_str = path.to_str().unwrap();
        let tmp_cfg = tempfile::tempdir().unwrap();

        // Warm the cache with the 5-commit layout.
        let layout1 = rebuild_layout_blocking(path_str, tmp_cfg.path()).expect("initial build");
        assert_eq!(layout1.nodes.len(), 5);

        // Add one commit so HEAD advances.
        {
            let git_repo = git2::Repository::open(&path).unwrap();
            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let parent = git_repo
                .find_commit(git_repo.head().unwrap().target().unwrap())
                .unwrap();
            let tree = git_repo
                .find_tree(git_repo.index().unwrap().write_tree().unwrap())
                .unwrap();
            git_repo
                .commit(Some("HEAD"), &sig, &sig, "extra", &tree, &[&parent])
                .unwrap();
        }

        // Refresh — must surface the new commit, not re-serve the stale
        // 5-node layout from cache.
        let layout2 =
            rebuild_layout_blocking(path_str, tmp_cfg.path()).expect("post-commit rebuild");
        assert_eq!(
            layout2.nodes.len(),
            6,
            "refresh must see the new commit after HEAD advances"
        );
    }
}
