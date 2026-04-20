//! Graph viewport, commit search, and commit detail commands.

use graph_builder::{Dag, GraphCommit, GraphLayout};
use tauri::State;
use tracing::instrument;

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

    let dag = Dag::build(&graph_commits);
    let layout = GraphLayout::compute(&dag);
    let vp = layout.viewport(0, graph_commits.len());

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Create a repo with `n` sequential single-parent commits.
    ///
    /// Replicated inline from `git-engine`'s test module because it is not
    /// re-exported. Kept minimal (a single linear chain) because the tests
    /// below only need predictable pagination semantics, not varied graph
    /// topology.
    fn create_repo_with_n_commits(dir: &tempfile::TempDir, n: usize) -> PathBuf {
        let path = dir.path().to_path_buf();
        let repo = git2::Repository::init(&path).expect("init");

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let mut parent_commit: Option<git2::Oid> = None;

        for i in 0..n {
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();

            let parents_vec: Vec<git2::Commit> = parent_commit
                .iter()
                .filter_map(|&oid| repo.find_commit(oid).ok())
                .collect();
            let parent_refs: Vec<&git2::Commit> = parents_vec.iter().collect();

            let msg = format!("Commit {}", i + 1);
            let oid = repo
                .commit(Some("HEAD"), &sig, &sig, &msg, &tree, &parent_refs)
                .unwrap();
            parent_commit = Some(oid);
        }

        path
    }

    #[test]
    fn build_graph_chunk_returns_offset_slice() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_n_commits(&dir, 50);
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
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_n_commits(&dir, 50);
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
        let dir = tempfile::TempDir::new().unwrap();
        let path = create_repo_with_n_commits(&dir, 50);
        let repo = git_engine::Repository::open(&path).unwrap();

        let chunk = build_graph_chunk(&repo, 100, 20).expect("chunk ok");
        assert!(chunk.nodes.is_empty());
        assert!(!chunk.has_more);
    }
}
