//! Graph viewport, commit search, and commit detail commands.

use std::path::PathBuf;

use graph_builder::{Dag, GraphCommit, GraphLayout};
use tauri::State;
use tracing::instrument;

use super::graph_cache::{GraphLayoutOptions, load_or_build_layout, persist_layout, ref_snapshot};
use super::helpers::*;
use crate::state::{AppState, RefSnapshot};

/// Ceiling on how far the incremental-refresh fast path walks a first-parent
/// chain before giving up and letting the full rebuild handle it. Keeps the
/// per-refresh work (done under the projects lock) bounded; larger jumps
/// (rebases, force-pushes, deep fast-forwards) legitimately full-rebuild.
const SIMPLE_ADVANCE_CAP: usize = 512;

/// Outcome of probing the active slot for a layout matching the requested
/// options: either a ready viewport slice, or the repo path to rebuild from.
enum SlotProbe {
    Sliced(Box<graph_builder::ViewportResult>),
    NeedsRebuild { path: String },
}

/// Return a paginated slice of the commit graph for virtual-scroll rendering.
///
/// # Parameters
/// - `offset`       – Zero-based row index of the first commit to include.
/// - `limit`        – Maximum number of rows to return.
/// - `first_parent` – When `true`, the graph follows only the first parent of
///   each commit (merges collapse onto the mainline). Defaults to `false`.
/// - `branch`       – When set, show only the history reachable from this
///   branch tip (local `main` or remote `origin/main`) instead of all refs.
///   Composes with `first_parent`.
/// - `max_lanes`    – Lane ceiling override, clamped to 4..=16. Defaults to
///   8. Wide windows can request more parallel lanes before the layout
///   starts recycling.
///
/// The active [`crate::state::ProjectSlot`] caches one layout at a time,
/// tagged with the options it was built with. When the requested options
/// match, this is a cheap in-memory slice; on a mode switch the layout is
/// rebuilt once (consulting the persistent per-variant disk cache) and
/// swapped into the slot, so subsequent scrolling is cheap again.
///
/// # Returns
/// A [`GraphViewport`] containing the layout nodes for the requested window.
#[tauri::command]
#[instrument(skip(state), name = "cmd::graph::viewport")]
pub async fn get_graph_viewport(
    offset: usize,
    limit: usize,
    first_parent: Option<bool>,
    branch: Option<String>,
    max_lanes: Option<u8>,
    state: State<'_, AppState>,
) -> Result<GraphViewport, String> {
    let options = GraphLayoutOptions {
        first_parent: first_parent.unwrap_or(false),
        branch,
        max_lanes: GraphLayoutOptions::normalize_max_lanes(max_lanes),
    };

    // Fast path: slice the slot's cached layout while holding the lock
    // briefly. The layout itself is not Clone/Send, so we compute the
    // viewport slice synchronously (it's array filtering, not DAG work).
    let probe = {
        let projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        let idx = active.ok_or_else(|| "No active project".to_string())?;
        let slot = projects
            .get(idx)
            .ok_or("Active project index out of bounds")?;
        let layout = slot.layout.as_ref().ok_or("No repository open")?;
        if slot.layout_options == options {
            SlotProbe::Sliced(Box::new(layout.viewport(offset, limit)))
        } else {
            SlotProbe::NeedsRebuild {
                path: slot.path.clone(),
            }
        }
    };

    let result = match probe {
        SlotProbe::Sliced(result) => *result,
        SlotProbe::NeedsRebuild { path } => {
            // Mode switch: rebuild off-thread (disk cache makes repeat
            // switches cheap), then swap the new layout into the slot.
            let config_dir = state.config_dir.clone();
            let path_clone = path.clone();
            let opts = options.clone();
            let layout = tokio::task::spawn_blocking(move || {
                let repo = git_engine::Repository::open(PathBuf::from(&path_clone))
                    .map_err(|e| e.to_string())?;
                let (layout, _was_cached) =
                    load_or_build_layout(&repo, &path_clone, &config_dir, &opts)?;
                Ok::<_, String>(layout)
            })
            .await
            .map_err(|e| e.to_string())??;

            let result = layout.viewport(offset, limit);

            // Re-acquire the lock and install the fresh layout. Only touch
            // the slot whose path still matches — a project switch that
            // raced with this call is a silent no-op.
            let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
            let active = state.active_index.lock().map_err(|e| e.to_string())?;
            if let Some(idx) = *active
                && let Some(slot) = projects.get_mut(idx)
                && slot.path == path
            {
                slot.layout = Some(layout);
                slot.layout_options = options;
            }
            result
        }
    };

    Ok(GraphViewport {
        nodes: result.nodes,
        lane_segments: result.lane_segments,
        merge_curves: result.merge_curves,
        total_count: result.total_count,
        offset: result.offset,
        visible_lane_count: result.visible_lane_count,
        total_lane_count: result.total_lane_count,
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
/// - `repo`    – Repository to walk.
/// - `offset`  – Zero-based index of the first commit to return.
/// - `limit`   – Maximum number of commits to include in the chunk.
/// - `options` – Layout mode (first-parent simplification, branch scope).
fn build_graph_chunk(
    repo: &git_engine::Repository,
    offset: usize,
    limit: usize,
    options: &GraphLayoutOptions,
) -> Result<GraphViewport, String> {
    // Walk one extra so we can detect whether more commits exist without
    // paying for a second round-trip.
    let commits = repo
        .walk_commits_with_options(offset, limit + 1, options.walk_options())
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
    let dag = if options.first_parent {
        Dag::build_first_parent(graph_commits)
    } else {
        Dag::build(graph_commits)
    };
    let layout = GraphLayout::compute_with_max_lanes(dag, options.lane_ceiling());
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
/// - `first_parent` – Follow only the first parent of each commit. Defaults
///   to `false`. Must match the mode of the layout the chunk extends.
/// - `branch` – Restrict the walk to history reachable from this branch tip.
///   Must match the mode of the layout the chunk extends.
/// - `max_lanes` – Lane ceiling override, clamped to 4..=16 (default 8).
///   Must match the mode of the layout the chunk extends.
///
/// # Returns
/// A [`GraphViewport`] whose `has_more` is `true` when additional commits
/// exist beyond the chunk.
#[tauri::command]
#[instrument(skip(state), name = "cmd::graph::load_chunk")]
pub async fn load_graph_chunk(
    offset: usize,
    limit: usize,
    first_parent: Option<bool>,
    branch: Option<String>,
    max_lanes: Option<u8>,
    state: State<'_, AppState>,
) -> Result<GraphViewport, String> {
    let repo_path = get_active_project_path(&state)?;
    let options = GraphLayoutOptions {
        first_parent: first_parent.unwrap_or(false),
        branch,
        max_lanes: GraphLayoutOptions::normalize_max_lanes(max_lanes),
    };

    tokio::task::spawn_blocking(move || {
        let repo = git_engine::Repository::open(repo_path).map_err(|e| e.to_string())?;
        build_graph_chunk(&repo, offset, limit, &options)
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
    options: &GraphLayoutOptions,
) -> Result<(GraphLayout, RefSnapshot), String> {
    let repo = git_engine::Repository::open(PathBuf::from(path)).map_err(|e| e.to_string())?;
    let (layout, _was_cached) = load_or_build_layout(&repo, path, config_dir, options)?;
    // Record the refs this layout was built from so a later `refresh_graph_layout`
    // can detect a simple advance against it.
    let refs = ref_snapshot(&repo);
    Ok((layout, refs))
}

/// Map a `git_engine::CommitInfo` into a `graph_builder::GraphCommit`.
fn commit_to_graph_commit(c: git_engine::CommitInfo) -> GraphCommit {
    GraphCommit {
        oid: c.oid,
        parents: c.parents,
        timestamp: c.timestamp,
        refs: c.refs,
        summary: c.summary,
        author: c.author,
        email: c.email,
    }
}

/// If exactly one ref changed OID between `old` and `new` (none added or
/// removed), return `(ref_name, old_oid, new_oid)`. Any other shape (a ref
/// created/deleted, several refs moved, or nothing moved) yields `None`.
fn single_ref_advance(old: &RefSnapshot, new: &RefSnapshot) -> Option<(String, String, String)> {
    if old.len() != new.len() {
        return None;
    }
    let mut moved: Option<(String, String, String)> = None;
    for (name, new_oid) in new {
        match old.get(name) {
            None => return None, // a ref was added (and one removed to keep len equal)
            Some(old_oid) if old_oid == new_oid => {}
            Some(old_oid) => {
                if moved.is_some() {
                    return None; // more than one ref moved
                }
                moved = Some((name.clone(), old_oid.clone(), new_oid.clone()));
            }
        }
    }
    moved
}

/// Attempt the incremental "simple advance" fast path for
/// [`refresh_graph_layout`]: when exactly one branch moved forward on top of
/// the current graph tip, patch the cached layout in place instead of
/// re-walking the whole graph.
///
/// Returns `Ok(true)` when it fully handled the refresh (layout patched, disk
/// cache updated, ref snapshot advanced); `Ok(false)` when the mutation isn't a
/// simple advance and the caller must full-rebuild. Only genuine lock failures
/// surface as `Err`.
///
/// The detect-and-patch runs under the `projects`+`active_index` lock: the git
/// walk is a capped first-parent hop and the array work is O(rows), so keeping
/// the old layout in the slot throughout avoids a `None` window that would
/// strand a concurrent viewport read. In debug builds the result is
/// cross-checked against a full rebuild so the fast path can never silently
/// diverge.
fn try_incremental_advance(state: &State<'_, AppState>) -> Result<bool, String> {
    // Resolve the active path, then read its recorded ref snapshot — each under
    // its own lock so we never hold two of AppState's mutexes simultaneously.
    let path = match get_active_project_path(state) {
        Ok(p) => p.to_string_lossy().into_owned(),
        Err(_) => return Ok(false),
    };
    let Some(old_refs) = state.layout_ref_snapshot(&path) else {
        return Ok(false); // no baseline yet — full rebuild will record one
    };
    let config_dir = state.config_dir.clone();

    let new_refs = {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        let Some(idx) = *active else {
            return Ok(false);
        };
        let Some(slot) = projects.get_mut(idx) else {
            return Ok(false);
        };
        // The fast path only reasons about the default full-graph view.
        if slot.path != path || slot.layout_options != GraphLayoutOptions::default() {
            return Ok(false);
        }
        let (Some(repo), Some(layout)) = (slot.repo.as_ref(), slot.layout.as_ref()) else {
            return Ok(false);
        };
        let Some(row0) = layout.nodes.first() else {
            return Ok(false);
        };
        if row0.lane != 0 {
            return Ok(false);
        }
        let (row0_oid, row0_ts) = (row0.oid.clone(), row0.timestamp);

        let new_refs = ref_snapshot(repo);
        let Some((name, old_oid, new_oid)) = single_ref_advance(&old_refs, &new_refs) else {
            return Ok(false);
        };
        // Only branch/remote tips anchor the graph, and the moved branch's old
        // tip must be exactly the current row-0 commit.
        if !(name.starts_with("refs/heads/") || name.starts_with("refs/remotes/"))
            || old_oid != row0_oid
        {
            return Ok(false);
        }

        let Some((commits, former_tip_refs)) = repo
            .simple_advance_commits(&old_oid, &new_oid, SIMPLE_ADVANCE_CAP)
            .map_err(|e| e.to_string())?
        else {
            return Ok(false);
        };
        // Every new commit must be at least as new as the old tip, so a full
        // rebuild would place them at the very top (see try_prepend docs).
        if commits.iter().any(|c| c.timestamp < row0_ts) {
            return Ok(false);
        }

        let gcs: Vec<GraphCommit> = commits.into_iter().map(commit_to_graph_commit).collect();
        let Some(inc) = layout.try_prepend_simple_advance(&gcs, former_tip_refs) else {
            return Ok(false);
        };

        // Dev-only correctness gate: the fast path must never diverge from a
        // full rebuild. Release builds trust the property-tested construction.
        #[cfg(debug_assertions)]
        {
            let full = super::graph_cache::build_fresh_layout(repo, &slot.layout_options)?;
            if let Some(diff) = graph_builder::structural_diff(&inc, &full) {
                panic!("incremental graph advance diverged from full rebuild: {diff}");
            }
        }

        // Serialize from a borrow (no clone) and flush off-thread, then install
        // the patched layout into the slot.
        persist_layout(repo, &path, &config_dir, &slot.layout_options, &inc);
        slot.layout = Some(inc);
        new_refs
    };

    state.store_layout_ref_snapshot(&path, new_refs);
    Ok(true)
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
    // Fast path: a plain commit / amend / fast-forward moves exactly one branch
    // forward on top of the current tip. Detect that and patch the cached
    // layout in place (O(new rows)) instead of re-walking up to 20K commits.
    // Anything else falls through to the full rebuild below.
    if try_incremental_advance(&state)? {
        return Ok(());
    }

    // Snapshot the path + config dir + the slot's current layout options so
    // we can drop the lock before doing the (potentially expensive) walk +
    // layout build off-thread. Rebuilding with the slot's own options keeps
    // the refreshed layout in the mode the user is currently viewing.
    let (path, config_dir, options) = {
        let projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        let idx = active.ok_or_else(|| "No active project".to_string())?;
        let slot = projects
            .get(idx)
            .ok_or_else(|| "Active project index out of bounds".to_string())?;
        (
            slot.path.clone(),
            state.config_dir.clone(),
            slot.layout_options.clone(),
        )
    };

    let path_clone = path.clone();
    let opts_clone = options.clone();
    let (layout, new_refs) = tokio::task::spawn_blocking(move || {
        rebuild_layout_blocking(&path_clone, &config_dir, &opts_clone)
    })
    .await
    .map_err(|e| e.to_string())??;

    // Re-acquire the lock and swap in the fresh layout. Only touch the slot
    // whose path still matches what we snapshotted — a project switch that
    // raced with this call is a silent no-op. Guard on the options still
    // matching too, so a concurrent mode switch isn't clobbered with a
    // layout built for the old mode.
    let installed = {
        let mut projects = state.projects.lock().map_err(|e| e.to_string())?;
        let active = state.active_index.lock().map_err(|e| e.to_string())?;
        if let Some(idx) = *active
            && let Some(slot) = projects.get_mut(idx)
            && slot.path == path
            && slot.layout_options == options
        {
            slot.layout = Some(layout);
            true
        } else {
            false
        }
    };
    // Record the refs this layout was built from so the next refresh can try
    // the incremental fast path against it.
    if installed {
        state.store_layout_ref_snapshot(&path, new_refs);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git_engine::test_support::create_repo_with_n_commits;

    fn refs(pairs: &[(&str, &str)]) -> RefSnapshot {
        pairs
            .iter()
            .map(|(n, o)| (n.to_string(), o.to_string()))
            .collect()
    }

    #[test]
    fn single_ref_advance_detects_one_moved_branch() {
        let old = refs(&[
            ("refs/heads/main", "aaaa"),
            ("refs/remotes/origin/main", "aaaa"),
        ]);
        let new = refs(&[
            ("refs/heads/main", "bbbb"),
            ("refs/remotes/origin/main", "aaaa"),
        ]);
        assert_eq!(
            single_ref_advance(&old, &new),
            Some((
                "refs/heads/main".to_string(),
                "aaaa".to_string(),
                "bbbb".to_string()
            ))
        );
    }

    #[test]
    fn single_ref_advance_rejects_multi_move_add_remove_and_noop() {
        let base = refs(&[("refs/heads/main", "aaaa"), ("refs/heads/dev", "cccc")]);
        // No change.
        assert_eq!(single_ref_advance(&base, &base), None);
        // Two refs moved.
        let two = refs(&[("refs/heads/main", "bbbb"), ("refs/heads/dev", "dddd")]);
        assert_eq!(single_ref_advance(&base, &two), None);
        // A ref added (count differs).
        let added = refs(&[
            ("refs/heads/main", "aaaa"),
            ("refs/heads/dev", "cccc"),
            ("refs/heads/feat", "eeee"),
        ]);
        assert_eq!(single_ref_advance(&base, &added), None);
        // One moved, one removed, one added (same count) → not a clean advance.
        let churn = refs(&[("refs/heads/main", "bbbb"), ("refs/heads/feat", "eeee")]);
        assert_eq!(single_ref_advance(&base, &churn), None);
    }

    #[test]
    fn build_graph_chunk_returns_offset_slice() {
        let (_dir, path) = create_repo_with_n_commits(50);
        let repo = git_engine::Repository::open(&path).unwrap();

        let chunk =
            build_graph_chunk(&repo, 10, 20, &GraphLayoutOptions::default()).expect("chunk ok");
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

        let chunk =
            build_graph_chunk(&repo, 40, 20, &GraphLayoutOptions::default()).expect("chunk ok");
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

        let chunk =
            build_graph_chunk(&repo, 100, 20, &GraphLayoutOptions::default()).expect("chunk ok");
        assert!(chunk.nodes.is_empty());
        assert!(!chunk.has_more);
    }

    #[test]
    fn build_graph_chunk_first_parent_collapses_merge() {
        let (_dir, path) = git_engine::test_support::create_repo_with_merged_branch();
        let repo = git_engine::Repository::open(&path).unwrap();

        // Default mode: 4 commits (merge + feature + 2 mainline), 2 lanes.
        let full = build_graph_chunk(&repo, 0, 100, &GraphLayoutOptions::default()).expect("full");
        assert_eq!(full.nodes.len(), 4);
        assert_eq!(full.total_lane_count, 2);

        // First-parent mode: the feature commit disappears and everything
        // sits on a single lane.
        let fp_opts = GraphLayoutOptions {
            first_parent: true,
            ..Default::default()
        };
        let fp = build_graph_chunk(&repo, 0, 100, &fp_opts).expect("fp");
        assert_eq!(fp.nodes.len(), 3);
        assert_eq!(fp.total_lane_count, 1);
        assert!(fp.nodes.iter().all(|n| n.lane == 0));
        assert!(!fp.nodes.iter().any(|n| n.summary == "feature work"));
    }

    #[test]
    fn build_graph_chunk_branch_scoped_walks_single_branch() {
        let (_dir, path) = git_engine::test_support::create_repo_with_merged_branch();

        // Add a "side" branch one commit ahead of the merge.
        let head_branch = {
            let git_repo = git2::Repository::open(&path).unwrap();
            let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
            let head = git_repo
                .find_commit(git_repo.head().unwrap().target().unwrap())
                .unwrap();
            let tree = head.tree().unwrap();
            let side_oid = git_repo
                .commit(None, &sig, &sig, "side work", &tree, &[&head])
                .unwrap();
            let side = git_repo.find_commit(side_oid).unwrap();
            git_repo.branch("side", &side, false).unwrap();
            git_repo.head().unwrap().shorthand().unwrap().to_string()
        };

        let repo = git_engine::Repository::open(&path).unwrap();

        // All refs: 5 commits. Scoped to the head branch: 4, no side work.
        let full = build_graph_chunk(&repo, 0, 100, &GraphLayoutOptions::default()).expect("full");
        assert_eq!(full.nodes.len(), 5);

        let scoped_opts = GraphLayoutOptions {
            branch: Some(head_branch.clone()),
            ..Default::default()
        };
        let scoped = build_graph_chunk(&repo, 0, 100, &scoped_opts).expect("scoped");
        assert_eq!(scoped.nodes.len(), 4);
        assert!(!scoped.nodes.iter().any(|n| n.summary == "side work"));

        // branch + first_parent = clean mainline of one branch.
        let clean_opts = GraphLayoutOptions {
            first_parent: true,
            branch: Some(head_branch),
            ..Default::default()
        };
        let clean = build_graph_chunk(&repo, 0, 100, &clean_opts).expect("clean");
        assert_eq!(clean.nodes.len(), 3);
        assert_eq!(clean.total_lane_count, 1);
        assert!(!clean.nodes.iter().any(|n| n.summary == "feature work"));
    }

    #[test]
    fn build_graph_chunk_respects_lane_ceiling() {
        // 6 branches diverging from one base → 6 concurrent lanes when the
        // ceiling allows, 4 when capped.
        let (_dir, path) = git_engine::test_support::create_repo_with_branches(&[
            "b0", "b1", "b2", "b3", "b4", "b5",
        ]);
        let repo = git_engine::Repository::open(&path).unwrap();

        let full = build_graph_chunk(&repo, 0, 100, &GraphLayoutOptions::default()).expect("full");
        assert_eq!(full.total_lane_count, 6);

        let capped_opts = GraphLayoutOptions {
            max_lanes: Some(4),
            ..Default::default()
        };
        let capped = build_graph_chunk(&repo, 0, 100, &capped_opts).expect("capped");
        assert_eq!(capped.total_lane_count, 4);
        assert!(capped.nodes.iter().all(|n| n.lane < 4));
    }

    #[test]
    fn build_graph_chunk_branch_scoped_unknown_branch_errors() {
        let (_dir, path) = create_repo_with_n_commits(3);
        let repo = git_engine::Repository::open(&path).unwrap();
        let opts = GraphLayoutOptions {
            branch: Some("does-not-exist".to_string()),
            ..Default::default()
        };
        assert!(build_graph_chunk(&repo, 0, 100, &opts).is_err());
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
        let (layout1, _refs1) =
            rebuild_layout_blocking(path_str, tmp_cfg.path(), &GraphLayoutOptions::default())
                .expect("initial build");
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
        let (layout2, _refs2) =
            rebuild_layout_blocking(path_str, tmp_cfg.path(), &GraphLayoutOptions::default())
                .expect("post-commit rebuild");
        assert_eq!(
            layout2.nodes.len(),
            6,
            "refresh must see the new commit after HEAD advances"
        );
    }
}
