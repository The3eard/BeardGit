//! Lane-based layout engine for the commit graph.
//!
//! [`GraphLayout::compute`] walks the [`Dag`] in insertion order (newest-first)
//! and assigns each node a **row** (its position in the list) and a **lane**
//! (its horizontal column in the graph). Lanes are recycled when a branch merges
//! back into another, keeping the graph compact. The maximum number of concurrent
//! lanes is capped at [`DEFAULT_MAX_LANES`] (overridable per layout via
//! [`GraphLayout::compute_with_max_lanes`]) to prevent the graph from spreading
//! too wide on repositories with many parallel branches.

// Lane layout

use crate::dag::{Dag, GraphCommit};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

/// Rows per [`ViewportIndex`] bucket. Sized so a default ~300-row viewport
/// touches only one or two buckets, while a whole-graph-spanning lane is listed
/// in just `ceil(rows / 512)` buckets.
const VIEWPORT_INDEX_BUCKET_ROWS: usize = 512;

/// Row-bucketed overlap index over a layout's `lane_segments` and
/// `merge_curves`, turning [`GraphLayout::viewport`]'s per-query full-array scan
/// into O(window buckets + overlap).
///
/// Bucket `b` covers rows `[b * bucket_rows, (b + 1) * bucket_rows)` and lists
/// the indices of every segment / curve whose row span overlaps it. A query
/// touches only the buckets its window covers.
///
/// Chosen over "sort by `start_row` + a max-span back-scan": the mainline lane
/// segment spans the entire graph, so a single max-span bound collapses the
/// back-scan to O(n) for windows near the bottom. Buckets bound the work by the
/// window size regardless of how long-lived a lane is — a long span is simply
/// listed in each bucket it crosses, and at most `lane_count` segments (plus a
/// handful of curves) cross any given row.
///
/// Derived data: held behind a `#[serde(skip)]` [`OnceLock`] on [`GraphLayout`]
/// and built lazily on the first `viewport` call, so the on-disk cache format is
/// unchanged and deserialized layouts rebuild it transparently.
#[derive(Debug, Clone, Default)]
pub struct ViewportIndex {
    pub(crate) bucket_rows: usize,
    /// `segment_buckets[b]` = indices into `lane_segments` overlapping bucket `b`.
    pub(crate) segment_buckets: Vec<Vec<u32>>,
    /// `curve_buckets[b]` = indices into `merge_curves` overlapping bucket `b`.
    pub(crate) curve_buckets: Vec<Vec<u32>>,
}

impl ViewportIndex {
    /// Build the index from a layout's node count, segments, and curves.
    pub(crate) fn build(
        node_count: usize,
        segments: &[LaneSegment],
        curves: &[MergeCurve],
    ) -> Self {
        let bucket_rows = VIEWPORT_INDEX_BUCKET_ROWS;
        let bucket_count = node_count.div_ceil(bucket_rows).max(1);
        let mut segment_buckets = vec![Vec::new(); bucket_count];
        let mut curve_buckets = vec![Vec::new(); bucket_count];

        for (i, seg) in segments.iter().enumerate() {
            let first = (seg.start_row / bucket_rows).min(bucket_count - 1);
            let last = (seg.end_row / bucket_rows).min(bucket_count - 1);
            for bucket in &mut segment_buckets[first..=last] {
                bucket.push(i as u32);
            }
        }
        for (i, c) in curves.iter().enumerate() {
            let min_row = c.from_row.min(c.to_row);
            let max_row = c.from_row.max(c.to_row);
            let first = (min_row / bucket_rows).min(bucket_count - 1);
            let last = (max_row / bucket_rows).min(bucket_count - 1);
            for bucket in &mut curve_buckets[first..=last] {
                bucket.push(i as u32);
            }
        }

        Self {
            bucket_rows,
            segment_buckets,
            curve_buckets,
        }
    }

    /// Indices (ascending, de-duplicated) whose bucket range intersects the
    /// touched buckets for window `[start, end)`. Callers still apply the exact
    /// overlap predicate — a bucket covers `bucket_rows` rows, wider than the
    /// window, so its list can contain non-overlapping entries.
    pub(crate) fn candidates(
        buckets: &[Vec<u32>],
        bucket_rows: usize,
        start: usize,
        end: usize,
    ) -> Vec<u32> {
        if start >= end || buckets.is_empty() {
            return Vec::new();
        }
        let b0 = start / bucket_rows;
        let b1 = ((end - 1) / bucket_rows).min(buckets.len() - 1);
        if b0 >= buckets.len() {
            return Vec::new();
        }
        let mut out: Vec<u32> = buckets[b0..=b1].iter().flatten().copied().collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Synchronization state of a lane segment relative to its remote tracking branch.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncState {
    /// Commit exists both locally and on the remote — thick solid line.
    Synced,
    /// Committed locally but not yet pushed — thin solid line.
    LocalOnly,
    /// Fetched from remote but not pulled/merged — thin dashed line.
    RemoteOnly,
    /// No remote tracking branch — renders as synced (default thick solid).
    Unknown,
}

/// Default maximum number of lanes before collapsing into the rightmost lane.
/// This prevents the graph from spreading infinitely to the right in large
/// repos. Callers with wider canvases can raise the ceiling per layout via
/// [`GraphLayout::compute_with_max_lanes`].
pub const DEFAULT_MAX_LANES: usize = 8;

/// A continuous vertical line segment in a lane.
///
/// Represents a run of rows where a single lane holds the same branch line,
/// from `start_row` through `end_row` (inclusive). Used by the canvas renderer
/// to draw vertical branch lines without per-row lookups.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LaneSegment {
    /// Horizontal lane index (0 = leftmost).
    pub lane: usize,
    /// First row of the segment (inclusive).
    pub start_row: usize,
    /// Last row of the segment (inclusive).
    pub end_row: usize,
    /// Color palette index for this lane's branch line.
    pub color_index: usize,
    /// `true` when this segment was cut short because the lane was recycled
    /// for a different branch. The renderer should draw a downward arrow at
    /// `end_row` to indicate the original branch continues further down.
    #[serde(default)]
    pub recycled: bool,
    /// Sync state relative to remote tracking branch.
    pub sync_state: SyncState,
    /// Unique ID for this segment group. Segments from different branches
    /// that share the same lane index have different group IDs.
    pub group_id: usize,
}

/// A cross-lane connection drawn as a bezier curve.
///
/// Emitted whenever a commit in one lane has a parent in a different lane,
/// representing a branch or merge edge that crosses horizontal lanes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeCurve {
    /// Lane of the child commit (where the curve starts).
    pub from_lane: usize,
    /// Row of the child commit.
    pub from_row: usize,
    /// Lane of the parent commit (where the curve ends).
    pub to_lane: usize,
    /// Row of the parent commit.
    pub to_row: usize,
    /// Color palette index for this curve.
    pub color_index: usize,
    /// Group ID of the child commit that generated this curve.
    pub group_id: usize,
}

/// A commit node with its final position (row + lane) and pre-computed edges.
///
/// This is the primary unit consumed by the canvas renderer; it contains
/// everything needed to draw the node and its outgoing edges without any
/// additional lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutNode {
    /// Full SHA-1 object ID of the commit.
    pub oid: String,
    /// Horizontal lane index (0 = leftmost).
    pub lane: usize,
    /// Vertical row index (0 = newest commit).
    pub row: usize,
    /// Branch and tag names pointing at this commit.
    pub refs: Vec<String>,
    /// First line of the commit message.
    pub summary: String,
    /// Author display name.
    pub author: String,
    /// Author email address.
    pub email: String,
    /// Unix author timestamp.
    pub timestamp: i64,
    /// `true` when the commit has more than one parent.
    pub is_merge: bool,
    /// `true` when the commit has no parents.
    pub is_root: bool,
    /// Group ID of the lane segment this node belongs to.
    pub segment_group: usize,
}

/// The complete lane-assigned layout for an entire commit graph.
///
/// Produced by [`GraphLayout::compute`] and then sliced by [`GraphLayout::viewport`]
/// before being sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLayout {
    /// All layout nodes in insertion order (newest-first).
    pub nodes: Vec<LayoutNode>,
    /// Number of lanes used across the whole graph (capped at the layout's
    /// lane ceiling, [`DEFAULT_MAX_LANES`] unless overridden).
    pub lane_count: usize,
    /// Continuous vertical lane segments, one per uninterrupted branch line.
    /// Sorted by `(lane, start_row)`. Used by the canvas renderer for efficient
    /// vertical line drawing.
    pub lane_segments: Vec<LaneSegment>,
    /// Cross-lane bezier curves connecting child commits to parents in different lanes.
    /// Emitted for every parent edge where `child.lane != parent.lane`.
    pub merge_curves: Vec<MergeCurve>,
    /// Lane index of the HEAD commit, if present in the graph.
    pub head_lane: Option<usize>,
    /// Lazily-built row-bucket index over `lane_segments` / `merge_curves` used
    /// by [`GraphLayout::viewport`]. Derived, not persisted: `#[serde(skip)]`
    /// keeps the on-disk cache format unchanged, and a deserialized layout
    /// rebuilds it on the first viewport query.
    #[serde(skip)]
    pub viewport_index: OnceLock<ViewportIndex>,
}

/// Tag each lane segment with its sync state by comparing local and remote ref positions.
fn tag_sync_states(nodes: &[LayoutNode], segments: &mut [LaneSegment]) {
    use std::collections::HashMap;

    // Step 1: Build ref → (lane, row) mapping
    let mut local_refs: HashMap<String, (usize, usize)> = HashMap::new();
    let mut remote_refs: HashMap<String, (usize, usize)> = HashMap::new();

    for node in nodes.iter().filter(|n| !n.refs.is_empty()) {
        for r in &node.refs {
            if let Some(name) = r.strip_prefix("refs/heads/") {
                local_refs.insert(name.to_string(), (node.lane, node.row));
            } else if let Some(rest) = r.strip_prefix("refs/remotes/") {
                // Strip the remote name: "origin/main" → "main"
                if let Some((_, name)) = rest.split_once('/') {
                    remote_refs.insert(name.to_string(), (node.lane, node.row));
                }
            }
        }
    }

    // Step 2: Build lane → sync boundaries
    struct LaneBounds {
        local_row: usize,
        remote_row: usize,
    }

    let mut lane_bounds: HashMap<usize, LaneBounds> = HashMap::new();

    for (name, &(local_lane, local_row)) in &local_refs {
        if let Some(&(remote_lane, remote_row)) = remote_refs.get(name) {
            if local_lane == remote_lane {
                // Same lane: direct comparison
                lane_bounds.insert(
                    local_lane,
                    LaneBounds {
                        local_row,
                        remote_row,
                    },
                );
            } else {
                // Different lanes: tag local lane with what we know
                lane_bounds.entry(local_lane).or_insert(LaneBounds {
                    local_row,
                    remote_row: local_row,
                });
            }
        }
    }

    // Tag lanes that only have a remote ref (fetched branches)
    for (name, &(remote_lane, remote_row)) in &remote_refs {
        if !local_refs.contains_key(name) {
            lane_bounds.entry(remote_lane).or_insert(LaneBounds {
                local_row: remote_row,
                remote_row,
            });
        }
    }

    // Step 3: Tag each segment
    for seg in segments.iter_mut() {
        if let Some(bounds) = lane_bounds.get(&seg.lane) {
            let top_row = seg.start_row;

            if bounds.local_row == bounds.remote_row {
                seg.sync_state = SyncState::Synced;
            } else if bounds.local_row < bounds.remote_row {
                // Local is ahead (lower row = more recent)
                if top_row < bounds.remote_row {
                    seg.sync_state = SyncState::LocalOnly;
                } else {
                    seg.sync_state = SyncState::Synced;
                }
            } else {
                // Remote is ahead
                if top_row < bounds.local_row {
                    seg.sync_state = SyncState::RemoteOnly;
                } else {
                    seg.sync_state = SyncState::Synced;
                }
            }
        }
    }
}

impl GraphLayout {
    /// Assign a row and lane to each node in the DAG.
    ///
    /// Lane allocation is capped at [`DEFAULT_MAX_LANES`]. When all lanes are
    /// full and a new one is needed, a stale lane is reclaimed (edges may
    /// overlap but the graph stays compact and readable).
    pub fn compute(dag: Dag) -> Self {
        Self::compute_with_max_lanes(dag, DEFAULT_MAX_LANES)
    }

    /// Like [`GraphLayout::compute`] with an explicit lane ceiling.
    ///
    /// `max_lanes` is clamped to at least 1. Wider windows can afford more
    /// parallel lanes before the layout starts recycling; narrow ones can
    /// lower the ceiling to keep the graph column compact.
    pub fn compute_with_max_lanes(dag: Dag, max_lanes_cap: usize) -> Self {
        let max_lanes_cap = max_lanes_cap.max(1);
        // Pre-extract the parents map so the second pass (merge curves) can
        // still answer "who are the parents of this oid?" after we have
        // moved every node's owned fields (`refs`, `summary`, `author`,
        // `email`) into its `LayoutNode`.
        let (ordered_nodes, parents_by_oid) = dag.into_ordered_nodes_with_parents();
        let mut active_lanes: Vec<Option<Arc<str>>> = Vec::new();
        let mut layout_nodes: Vec<LayoutNode> = Vec::new();
        let mut position: std::collections::HashMap<Arc<str>, (usize, usize)> =
            std::collections::HashMap::new();

        // Tracks the row at which each lane's current segment began.
        // `None` means the lane slot is currently unused.
        let mut lane_start_row: Vec<Option<usize>> = Vec::new();
        let mut lane_segments: Vec<LaneSegment> = Vec::new();

        // Group tracking: unique ID per continuous branch tenure on a lane.
        // Different branches sharing the same lane index get different group IDs.
        let mut next_group_id: usize = 0;
        let mut lane_group: Vec<usize> = Vec::new();

        let find_lane = |lanes: &[Option<Arc<str>>], oid: &str| -> Option<usize> {
            lanes.iter().position(|slot| slot.as_deref() == Some(oid))
        };

        /// Allocate a lane for a new branch.
        ///
        /// `hint` is the lane of the commit that triggered the allocation
        /// (the child's lane when allocating a merge parent). When several
        /// candidate lanes are available, the one **nearest to the hint**
        /// is chosen so merge curves stay short and cross fewer lanes.
        /// `hint = None` (placing a brand-new tip with no child context)
        /// reproduces the historical behaviour: lowest free slot first,
        /// highest stale lane reclaimed at the cap.
        ///
        /// Priority:
        /// 1. Reuse a free (None) slot — the one nearest to `hint`
        ///    (ties prefer the lower index; deterministic).
        /// 2. Append a new lane if under `max_lanes_cap`.
        /// 3. At the cap, reclaim a **stale** lane — one whose OID is also
        ///    tracked by another lane (a duplicate that will never get its
        ///    own commit node) — again nearest to `hint`.
        /// 4. Last resort: overwrite the highest lane (original behavior).
        ///
        /// Returns `(lane_index, was_recycled)`.
        #[allow(clippy::too_many_arguments)]
        fn alloc_lane(
            lanes: &mut Vec<Option<Arc<str>>>,
            lane_start_row: &mut Vec<Option<usize>>,
            lane_segments: &mut Vec<LaneSegment>,
            lane_group: &mut Vec<usize>,
            next_group_id: &mut usize,
            oid: Arc<str>,
            current_row: usize,
            hint: Option<usize>,
            max_lanes_cap: usize,
        ) -> (usize, bool) {
            // 1. Reuse the free slot nearest to the hint. With no hint the
            //    distance to lane 0 is the index itself, i.e. lowest-first.
            let target = hint.unwrap_or(0);
            let free_idx = (0..lanes.len())
                .filter(|&i| lanes[i].is_none())
                .min_by_key(|&i| (i.abs_diff(target), i));
            if let Some(idx) = free_idx {
                lanes[idx] = Some(oid);
                while lane_start_row.len() <= idx {
                    lane_start_row.push(None);
                }
                while lane_group.len() <= idx {
                    lane_group.push(0);
                }
                lane_start_row[idx] = Some(current_row);
                lane_group[idx] = *next_group_id;
                *next_group_id += 1;
                return (idx, false);
            }
            // 2. Under the cap — append new lane
            if lanes.len() < max_lanes_cap {
                lanes.push(Some(oid));
                lane_start_row.push(Some(current_row));
                lane_group.push(*next_group_id);
                *next_group_id += 1;
                return (lanes.len() - 1, false);
            }
            // 3. At the cap — try to reclaim a stale (duplicate-OID) lane.
            //    A lane is stale if its OID appears in at least one other lane.
            //    With a hint, pick the stale lane nearest to it; without one,
            //    keep the historical highest-index choice so lower lanes stay
            //    visually stable.
            //
            //    The occurrence table is built once over the active lanes
            //    (O(lane cap)) so each candidate check is O(1).
            let mut occ: std::collections::HashMap<&str, u8> =
                std::collections::HashMap::with_capacity(lanes.len());
            for o in lanes.iter().flatten() {
                *occ.entry(o.as_ref()).or_insert(0) += 1;
            }
            let is_stale = |i: usize| {
                lanes[i]
                    .as_ref()
                    .map(|o| occ.get(o.as_ref()).copied().unwrap_or(0) > 1)
                    .unwrap_or(false)
            };
            let stale_idx: Option<usize> = match hint {
                Some(h) => (0..lanes.len())
                    .filter(|&i| is_stale(i))
                    .min_by_key(|&i| (i.abs_diff(h), i)),
                None => (0..lanes.len()).rev().find(|&i| is_stale(i)),
            };
            let reclaim_idx = stale_idx.unwrap_or(lanes.len() - 1);
            // Close the existing segment before overwriting — mark as recycled
            // so the renderer draws a continuation arrow. Guard `start <
            // current_row`: an octopus merge with more parents than the lane
            // cap can reclaim a lane allocated at THIS same row, which would
            // emit an inverted segment (end_row = current_row - 1 < start_row).
            if let Some(start) = lane_start_row[reclaim_idx]
                && start < current_row
            {
                lane_segments.push(LaneSegment {
                    lane: reclaim_idx,
                    start_row: start,
                    end_row: current_row - 1,
                    color_index: reclaim_idx,
                    recycled: true,
                    sync_state: SyncState::Unknown,
                    group_id: lane_group[reclaim_idx],
                });
            }
            lanes[reclaim_idx] = Some(oid);
            lane_start_row[reclaim_idx] = Some(current_row);
            lane_group[reclaim_idx] = *next_group_id;
            *next_group_id += 1;
            (reclaim_idx, true)
        }

        let mut max_lanes: usize = 0;

        for (row, dag_node) in ordered_nodes.into_iter().enumerate() {
            let lane = if let Some(idx) = find_lane(&active_lanes, &dag_node.oid) {
                idx
            } else {
                // A brand-new tip has no child context — no affinity hint.
                let (idx, _) = alloc_lane(
                    &mut active_lanes,
                    &mut lane_start_row,
                    &mut lane_segments,
                    &mut lane_group,
                    &mut next_group_id,
                    Arc::clone(&dag_node.oid),
                    row,
                    None,
                    max_lanes_cap,
                );
                idx
            };

            // Ensure tracking vecs cover this lane index
            while lane_start_row.len() <= lane {
                lane_start_row.push(None);
            }
            while lane_group.len() <= lane {
                lane_group.push(0);
            }
            // Start tracking this lane's segment if not already started
            if lane_start_row[lane].is_none() {
                lane_start_row[lane] = Some(row);
                lane_group[lane] = next_group_id;
                next_group_id += 1;
            }

            active_lanes[lane] = Some(Arc::clone(&dag_node.oid));

            if dag_node.parents.is_empty() {
                // Root commit: close this lane's segment
                if let Some(start) = lane_start_row[lane].take() {
                    lane_segments.push(LaneSegment {
                        lane,
                        start_row: start,
                        end_row: row,
                        color_index: lane,
                        recycled: false,
                        sync_state: SyncState::Unknown,
                        group_id: lane_group[lane],
                    });
                }
                active_lanes[lane] = None;
            } else {
                for (i, parent_oid) in dag_node.parents.iter().enumerate() {
                    if i == 0 {
                        active_lanes[lane] = Some(Arc::clone(parent_oid));
                    } else {
                        let already_assigned = find_lane(&active_lanes, parent_oid).is_some();
                        if !already_assigned {
                            // Hint with the merge commit's own lane so the
                            // parent lands as close as possible and the
                            // resulting curve stays short.
                            alloc_lane(
                                &mut active_lanes,
                                &mut lane_start_row,
                                &mut lane_segments,
                                &mut lane_group,
                                &mut next_group_id,
                                Arc::clone(parent_oid),
                                row,
                                Some(lane),
                                max_lanes_cap,
                            );
                        }
                    }
                }
            }

            if active_lanes.len() > max_lanes {
                max_lanes = active_lanes.len();
            }

            position.insert(Arc::clone(&dag_node.oid), (lane, row));

            layout_nodes.push(LayoutNode {
                oid: dag_node.oid.to_string(),
                lane,
                row,
                // Move owned fields out of the consumed DagNode rather than
                // cloning them. Saves ~5 owned-field clones per commit.
                refs: dag_node.refs,
                summary: dag_node.summary,
                author: dag_node.author,
                email: dag_node.email,
                timestamp: dag_node.timestamp,
                is_merge: dag_node.is_merge,
                is_root: dag_node.is_root,
                segment_group: lane_group[lane],
            });
        }

        // Close any segments that are still open after the main loop
        let last_row = layout_nodes.len().saturating_sub(1);
        for (lane_idx, start_opt) in lane_start_row.iter_mut().enumerate() {
            if let Some(start) = start_opt.take() {
                lane_segments.push(LaneSegment {
                    lane: lane_idx,
                    start_row: start,
                    end_row: last_row,
                    color_index: lane_idx,
                    recycled: false,
                    sync_state: SyncState::Unknown,
                    group_id: lane_group[lane_idx],
                });
            }
        }

        // Second pass: compute merge curves for cross-lane parent edges.
        // The DAG itself has been consumed; `parents_by_oid` carries
        // exactly what this pass needs.
        let mut merge_curves: Vec<MergeCurve> = Vec::new();
        for layout_node in &layout_nodes {
            let oid_arc: Arc<str> = Arc::from(layout_node.oid.as_str());
            let Some(parents) = parents_by_oid.get(&oid_arc) else {
                continue;
            };
            for parent_oid in parents {
                if let Some(&(parent_lane, parent_row)) = position.get(parent_oid)
                    && layout_node.lane != parent_lane
                {
                    merge_curves.push(MergeCurve {
                        from_lane: layout_node.lane,
                        from_row: layout_node.row,
                        to_lane: parent_lane,
                        to_row: parent_row,
                        color_index: layout_node.lane,
                        group_id: layout_node.segment_group,
                    });
                }
            }
        }

        // Tag lane segments with sync state based on local/remote ref pairs
        tag_sync_states(&layout_nodes, &mut lane_segments);

        // Detect the lane of the HEAD commit, if any
        let head_lane = layout_nodes
            .iter()
            .find(|n| n.refs.iter().any(|r| r == "HEAD"))
            .map(|n| n.lane);

        GraphLayout {
            nodes: layout_nodes,
            lane_count: max_lanes.min(max_lanes_cap),
            lane_segments,
            merge_curves,
            head_lane,
            viewport_index: OnceLock::new(),
        }
    }

    /// Incrementally prepend a "simple advance" onto this layout, producing the
    /// layout a full [`GraphLayout::compute`] would yield — without re-walking
    /// the graph.
    ///
    /// A *simple advance* is the common case where a single branch moved
    /// forward by one or more commits on top of the current graph tip (a plain
    /// commit, an amend that adds nothing new above, or a linear fast-forward):
    /// the new commits form a first-parent chain of single-parent commits whose
    /// oldest parent is this layout's row-0 commit, and that row-0 commit sits
    /// on lane 0. Under exactly those conditions a full rebuild is provably
    /// equal to "prepend the new rows on lane 0, shift every existing row down
    /// by N": the new commits are the globally-newest commits, so the walk
    /// after them re-enters the identical state that produced the old layout,
    /// leaving lane assignment, group ids, segments and curves unchanged apart
    /// from a uniform row shift.
    ///
    /// # Parameters
    /// - `new_commits` — the new commits **newest-first** (`new_commits[0]` is
    ///   the branch tip). Each must have exactly one parent; consecutive
    ///   entries must chain (`new_commits[i].parents[0] == new_commits[i+1].oid`)
    ///   and the last one's parent must be `self.nodes[0].oid`.
    /// - `former_tip_refs` — the refs (short names) that now point at the former
    ///   row-0 commit after the branch moved off it. Applied to that (shifted)
    ///   node so ref chips track the move; the moved branch/HEAD labels ride
    ///   along on the new commits' own refs.
    ///
    /// Returns `None` if the inputs don't match the strict simple-advance shape.
    /// Callers must treat `None` as "fall back to a full rebuild" — correctness
    /// never depends on this fast path succeeding.
    ///
    /// The result is cross-checked against a full rebuild by
    /// [`structural_diff`] in the caller's debug builds and by a property test
    /// (`incremental_prepend_matches_full_rebuild_across_topologies`).
    pub fn try_prepend_simple_advance(
        &self,
        new_commits: &[GraphCommit],
        former_tip_refs: Vec<String>,
    ) -> Option<GraphLayout> {
        // ── Precondition checks — bail (→ full rebuild) on anything off-shape.
        let first = self.nodes.first()?;
        if first.lane != 0 {
            return None;
        }
        if new_commits.is_empty() {
            return None;
        }
        for c in new_commits {
            if c.parents.len() != 1 {
                return None; // a merge (or root) in the range → not a simple advance
            }
        }
        for pair in new_commits.windows(2) {
            if pair[0].parents[0] != pair[1].oid {
                return None; // not a contiguous first-parent chain
            }
        }
        if new_commits.last()?.parents[0] != first.oid {
            return None; // chain doesn't attach to the current tip
        }

        let n = new_commits.len();
        let group = first.segment_group; // the lane-0 tip tenure (== 0 in practice)

        // ── Nodes: N new rows on lane 0, then every old node shifted down by N.
        let mut nodes: Vec<LayoutNode> = Vec::with_capacity(n + self.nodes.len());
        for (i, c) in new_commits.iter().enumerate() {
            nodes.push(LayoutNode {
                oid: c.oid.clone(),
                lane: 0,
                row: i,
                refs: c.refs.clone(),
                summary: c.summary.clone(),
                author: c.author.clone(),
                email: c.email.clone(),
                timestamp: c.timestamp,
                is_merge: c.parents.len() > 1, // always false here (checked above)
                is_root: c.parents.is_empty(), // always false here
                segment_group: group,
            });
        }
        for (i, old) in self.nodes.iter().enumerate() {
            let mut node = old.clone();
            node.row += n;
            if i == 0 {
                // The former tip: its moved branch/HEAD labels now live on the
                // new tip, so replace its refs with what still points at it.
                node.refs = former_tip_refs.clone();
            }
            nodes.push(node);
        }

        // ── Segments: shift by N. The tip's own segment — the one whose
        // group id matches the row-0 node — keeps start_row 0 and just extends
        // downward to cover the new rows; every other segment shifts start and
        // end uniformly. (Identifying the tip segment by group id, not by
        // `start_row == 0`, matters: a merge at row 0 opens a *second* segment
        // that also starts at row 0 but must still shift down by N.) Reset sync
        // state to Unknown so the re-tag below reproduces a fresh compute
        // exactly.
        let mut lane_segments: Vec<LaneSegment> = self
            .lane_segments
            .iter()
            .map(|seg| {
                let mut s = seg.clone();
                s.end_row += n;
                if seg.group_id != group {
                    s.start_row += n;
                }
                s.sync_state = SyncState::Unknown;
                s
            })
            .collect();

        // ── Curves: shift every endpoint down by N (no new curves — the new
        // commits are single-parent on lane 0, so they cross no lanes).
        let merge_curves: Vec<MergeCurve> = self
            .merge_curves
            .iter()
            .map(|c| MergeCurve {
                from_row: c.from_row + n,
                to_row: c.to_row + n,
                ..c.clone()
            })
            .collect();

        tag_sync_states(&nodes, &mut lane_segments);

        let head_lane = nodes
            .iter()
            .find(|node| node.refs.iter().any(|r| r == "HEAD"))
            .map(|node| node.lane);

        Some(GraphLayout {
            nodes,
            lane_count: self.lane_count,
            lane_segments,
            merge_curves,
            head_lane,
            viewport_index: OnceLock::new(),
        })
    }
}

/// Compare two layouts for **structural** equality, returning `Some(reason)`
/// describing the first difference or `None` when they match.
///
/// Used to cross-check [`GraphLayout::try_prepend_simple_advance`] against a
/// full [`GraphLayout::compute`] — in the caller's debug builds via
/// `debug_assert!` and in the incremental-prepend property test. Ref lists are
/// compared as sets (order carries no rendering meaning); everything else is
/// compared exactly, including `lane_segments` / `merge_curves` ordering, since
/// the renderer consumes those vectors positionally.
pub fn structural_diff(a: &GraphLayout, b: &GraphLayout) -> Option<String> {
    if a.nodes.len() != b.nodes.len() {
        return Some(format!("node count {} != {}", a.nodes.len(), b.nodes.len()));
    }
    if a.lane_count != b.lane_count {
        return Some(format!("lane_count {} != {}", a.lane_count, b.lane_count));
    }
    if a.head_lane != b.head_lane {
        return Some(format!("head_lane {:?} != {:?}", a.head_lane, b.head_lane));
    }
    for (i, (x, y)) in a.nodes.iter().zip(b.nodes.iter()).enumerate() {
        if x.oid != y.oid
            || x.lane != y.lane
            || x.row != y.row
            || x.summary != y.summary
            || x.author != y.author
            || x.email != y.email
            || x.timestamp != y.timestamp
            || x.is_merge != y.is_merge
            || x.is_root != y.is_root
            || x.segment_group != y.segment_group
        {
            return Some(format!("node[{i}] differs: {x:?} != {y:?}"));
        }
        let mut xr = x.refs.clone();
        let mut yr = y.refs.clone();
        xr.sort();
        yr.sort();
        if xr != yr {
            return Some(format!("node[{i}] refs {xr:?} != {yr:?}"));
        }
    }
    if a.lane_segments != b.lane_segments {
        return Some(format!(
            "lane_segments differ ({} vs {})",
            a.lane_segments.len(),
            b.lane_segments.len()
        ));
    }
    if a.merge_curves != b.merge_curves {
        return Some(format!(
            "merge_curves differ ({} vs {})",
            a.merge_curves.len(),
            b.merge_curves.len()
        ));
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{Dag, GraphCommit};

    fn commit(oid: &str, parents: &[&str]) -> GraphCommit {
        GraphCommit {
            oid: oid.to_string(),
            parents: parents.iter().map(|s| s.to_string()).collect(),
            timestamp: 0,
            refs: Vec::new(),
            summary: format!("commit {}", oid),
            author: String::new(),
            email: String::new(),
        }
    }

    fn make_commit(oid: &str, parents: &[&str], refs: &[&str], summary: &str) -> GraphCommit {
        GraphCommit {
            oid: oid.to_string(),
            parents: parents.iter().map(|s| s.to_string()).collect(),
            timestamp: 0,
            refs: refs.iter().map(|s| s.to_string()).collect(),
            summary: summary.to_string(),
            author: String::new(),
            email: String::new(),
        }
    }

    #[test]
    fn test_linear_layout_single_lane() {
        let commits = vec![commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);

        assert_eq!(layout.nodes.len(), 3);
        assert_eq!(layout.lane_count, 1, "linear history should fit in 1 lane");

        for (i, node) in layout.nodes.iter().enumerate() {
            assert_eq!(node.lane, 0);
            assert_eq!(node.row, i);
        }
    }

    #[test]
    fn test_branch_creates_new_lane() {
        // Merge commit with two parents → needs at least 2 lanes
        let commits = vec![
            commit("m", &["b1", "b2"]),
            commit("b1", &["base"]),
            commit("b2", &["base"]),
            commit("base", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);

        assert!(
            layout.lane_count >= 2,
            "merge history should use at least 2 lanes, got {}",
            layout.lane_count
        );
    }

    #[test]
    fn test_first_parent_layout_collapses_merge_to_single_lane() {
        // First-parent walk of a merged-branch history: b2 (only reachable
        // through m's second parent) is absent from the commit list.
        let commits = vec![
            commit("m", &["b1", "b2"]),
            commit("b1", &["base"]),
            commit("base", &[]),
        ];
        let dag = Dag::build_first_parent(commits);
        let layout = GraphLayout::compute(dag);

        assert_eq!(
            layout.lane_count, 1,
            "first-parent history must collapse into a single lane"
        );
        assert!(
            layout.nodes.iter().all(|n| n.lane == 0),
            "every mainline commit should sit on lane 0"
        );
        assert!(
            layout.merge_curves.is_empty(),
            "no cross-lane curves in first-parent mode"
        );
        assert!(
            !layout.nodes.iter().any(|n| n.oid == "b2"),
            "merged-branch commit must not appear"
        );
        // The merge commit keeps its marker.
        let m = layout.nodes.iter().find(|n| n.oid == "m").unwrap();
        assert!(m.is_merge);
    }

    #[test]
    fn test_merge_curves_have_valid_coordinates() {
        let commits = vec![
            commit("m", &["b1", "b2"]),
            commit("b1", &["base"]),
            commit("b2", &["base"]),
            commit("base", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);

        for curve in &layout.merge_curves {
            assert!(
                curve.to_row > curve.from_row,
                "parent row {} should be > child row {} (from lane {} to lane {})",
                curve.to_row,
                curve.from_row,
                curve.from_lane,
                curve.to_lane
            );
            assert_ne!(
                curve.from_lane, curve.to_lane,
                "merge curve should connect different lanes"
            );
        }
    }

    #[test]
    fn test_empty_dag() {
        let dag = Dag::build(vec![]);
        let layout = GraphLayout::compute(dag);
        assert_eq!(layout.nodes.len(), 0);
        assert_eq!(layout.lane_count, 0);
    }

    #[test]
    fn test_linear_history_produces_one_segment() {
        let commits = vec![commit("c", &["b"]), commit("b", &["a"]), commit("a", &[])];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        assert_eq!(layout.lane_segments.len(), 1);
        assert_eq!(
            layout.lane_segments[0],
            LaneSegment {
                lane: 0,
                start_row: 0,
                end_row: 2,
                color_index: 0,
                recycled: false,
                sync_state: SyncState::Unknown,
                group_id: 0,
            }
        );
        assert!(layout.merge_curves.is_empty());
    }

    #[test]
    fn test_branch_and_merge_produces_segments_and_curve() {
        let commits = vec![
            commit("m", &["b1", "b2"]),
            commit("b1", &["base"]),
            commit("b2", &["base"]),
            commit("base", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        assert!(
            layout.lane_segments.len() >= 2,
            "got {} segments",
            layout.lane_segments.len()
        );
        assert!(
            !layout.merge_curves.is_empty(),
            "expected merge curves for cross-lane edge"
        );
    }

    #[test]
    fn test_no_segments_for_empty_graph() {
        let dag = Dag::build(vec![]);
        let layout = GraphLayout::compute(dag);
        assert!(layout.lane_segments.is_empty());
        assert!(layout.merge_curves.is_empty());
    }

    #[test]
    fn test_single_commit_produces_one_segment() {
        let commits = vec![commit("a", &[])];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        assert_eq!(layout.lane_segments.len(), 1);
        assert_eq!(
            layout.lane_segments[0],
            LaneSegment {
                lane: 0,
                start_row: 0,
                end_row: 0,
                color_index: 0,
                recycled: false,
                sync_state: SyncState::Unknown,
                group_id: 0,
            }
        );
    }

    #[test]
    fn test_segment_continuity_no_gaps() {
        let commits = vec![
            commit("e", &["d"]),
            commit("d", &["c"]),
            commit("c", &["b"]),
            commit("b", &["a"]),
            commit("a", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        let lane0_segs: Vec<&LaneSegment> = layout
            .lane_segments
            .iter()
            .filter(|s| s.lane == 0)
            .collect();
        assert_eq!(lane0_segs.len(), 1);
        assert_eq!(lane0_segs[0].start_row, 0);
        assert_eq!(lane0_segs[0].end_row, 4);
    }

    #[test]
    fn test_stale_lanes_freed_on_common_ancestor() {
        // Two branches merge into the same base — the duplicate lane
        // tracking `base` should be freed when `base` is placed.
        //
        //   m (merge b1, b2)     row 0, lane 0
        //   b1                   row 1, lane 0
        //   b2                   row 2, lane 1
        //   base                 row 3, lane 0  ← lane 1 should be freed here
        let commits = vec![
            commit("m", &["b1", "b2"]),
            commit("b1", &["base"]),
            commit("b2", &["base"]),
            commit("base", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);

        // `base` must be placed in exactly one lane (lane 0), not duplicated
        let base_nodes: Vec<_> = layout.nodes.iter().filter(|n| n.oid == "base").collect();
        assert_eq!(base_nodes.len(), 1);
        assert_eq!(base_nodes[0].lane, 0);

        // Lane count should be exactly 2 (lane 0 + lane 1), with lane 1
        // freed when base is reached — not stuck at 2 forever.
        assert_eq!(layout.lane_count, 2);
    }

    #[test]
    fn test_stale_lanes_freed_at_common_ancestor_multiple_branches() {
        // Multiple branches converge on the same base. Stale lanes are
        // freed when the common ancestor is processed. The peak lane count
        // reflects the maximum concurrent branches, but freed lanes are
        // available for reuse by later branches.
        //
        //   m2 (merge m1, f2)    row 0
        //   f2                   row 1
        //   m1 (merge base, f1)  row 2
        //   f1                   row 3
        //   base                 row 4 ← stale lanes freed here
        let commits = vec![
            make_commit("m2", &["m1", "f2"], &[], "merge f2"),
            make_commit("f2", &["base"], &[], "f2 work"),
            make_commit("m1", &["base", "f1"], &[], "merge f1"),
            make_commit("f1", &["base"], &[], "f1 work"),
            make_commit("base", &[], &[], "initial"),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        // Peak is 3 (lanes 0, 1, 2 active at rows 2-3).
        // All lanes are freed once `base` is processed.
        assert!(
            layout.lane_count <= 3,
            "Expected <= 3 lanes, got {}",
            layout.lane_count
        );
        // Verify base is placed in exactly one lane
        let base_nodes: Vec<_> = layout.nodes.iter().filter(|n| n.oid == "base").collect();
        assert_eq!(base_nodes.len(), 1);
    }

    #[test]
    fn test_lanes_are_recycled() {
        // Create a history with sequential branches that merge back:
        // m2 (merge f2 into main)
        // f2-commit
        // m1 (merge f1 into main)
        // f1-commit
        // base
        let commits = vec![
            make_commit("m2", &["m1", "f2"], &["main"], "merge f2"),
            make_commit("f2", &["base"], &["feature2"], "f2 work"),
            make_commit("m1", &["base", "f1"], &[], "merge f1"),
            make_commit("f1", &["base"], &["feature1"], "f1 work"),
            make_commit("base", &[], &[], "initial"),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        // After f1's lane merges back, f2 should reuse it
        // Lane count should be <= 3, not grow unbounded
        assert!(
            layout.lane_count <= 3,
            "Expected <= 3 lanes, got {}",
            layout.lane_count
        );
    }

    #[test]
    fn test_merge_parent_prefers_nearest_free_lane() {
        // Build a state where, when merge commit `c` (lane 2) needs a lane
        // for its second parent `f`, both lane 0 and lane 3 are free.
        // Nearest-lane affinity must pick lane 3 (distance 1) instead of the
        // historical first-free choice, lane 0 (distance 2).
        //
        //   row 0  t0 (tip, lane 0)  → a
        //   row 1  t1 (tip, lane 1)  → b
        //   row 2  t2 (tip, lane 2)  → c
        //   row 3  t3 (tip, lane 3)  → d
        //   row 4  a  (root, frees lane 0)
        //   row 5  d  (root, frees lane 3)
        //   row 6  c  (merge of e, f — second parent f allocates here)
        //   row 7  f  → e
        //   row 8  b  → e
        //   row 9  e  (root)
        let commits = vec![
            commit("t0", &["a"]),
            commit("t1", &["b"]),
            commit("t2", &["c"]),
            commit("t3", &["d"]),
            commit("a", &[]),
            commit("d", &[]),
            commit("c", &["e", "f"]),
            commit("f", &["e"]),
            commit("b", &["e"]),
            commit("e", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);

        let c = layout.nodes.iter().find(|n| n.oid == "c").unwrap();
        assert_eq!(c.lane, 2, "merge child sits on lane 2");
        let f = layout.nodes.iter().find(|n| n.oid == "f").unwrap();
        assert_eq!(
            f.lane, 3,
            "second parent must take the free lane nearest to the child \
             (lane 3, distance 1) instead of the first free slot (lane 0)"
        );

        // The resulting merge curve spans a single lane.
        let curve = layout
            .merge_curves
            .iter()
            .find(|mc| mc.from_row == c.row && mc.to_row == f.row)
            .expect("curve from c to f");
        assert_eq!(curve.from_lane.abs_diff(curve.to_lane), 1);
    }

    #[test]
    fn test_merge_parent_nearest_lane_tie_prefers_lower_index() {
        // Same shape as above but the merge child sits on lane 2 with free
        // lanes 1 and 3 — both at distance 1. The lower index must win so
        // the layout stays deterministic and compact.
        //
        //   row 0  t0 (lane 0) → a
        //   row 1  t1 (lane 1) → b
        //   row 2  t2 (lane 2) → c
        //   row 3  t3 (lane 3) → d
        //   row 4  b  (root, frees lane 1)
        //   row 5  d  (root, frees lane 3)
        //   row 6  c  (merge of e, f)
        let commits = vec![
            commit("t0", &["a"]),
            commit("t1", &["b"]),
            commit("t2", &["c"]),
            commit("t3", &["d"]),
            commit("b", &[]),
            commit("d", &[]),
            commit("c", &["e", "f"]),
            commit("f", &["e"]),
            commit("a", &["e"]),
            commit("e", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);

        let f = layout.nodes.iter().find(|n| n.oid == "f").unwrap();
        assert_eq!(
            f.lane, 1,
            "equidistant free lanes resolve to the lower index"
        );
    }

    /// 12 parallel tips: tip i → root r_i, all roots last. Forces 12
    /// concurrent lanes if the ceiling allows them.
    fn wide_commits() -> Vec<GraphCommit> {
        let mut commits = Vec::new();
        for i in 0..12 {
            let tip = format!("t{i}");
            let root = format!("r{i}");
            commits.push(GraphCommit {
                oid: tip.clone(),
                parents: vec![root],
                timestamp: 0,
                refs: Vec::new(),
                summary: format!("commit {tip}"),
                author: String::new(),
                email: String::new(),
            });
        }
        for i in 0..12 {
            let root = format!("r{i}");
            commits.push(GraphCommit {
                oid: root.clone(),
                parents: Vec::new(),
                timestamp: 0,
                refs: Vec::new(),
                summary: format!("commit {root}"),
                author: String::new(),
                email: String::new(),
            });
        }
        commits
    }

    #[test]
    fn test_default_ceiling_caps_lanes_at_eight() {
        let dag = Dag::build(wide_commits());
        let layout = GraphLayout::compute(dag);
        assert_eq!(layout.lane_count, DEFAULT_MAX_LANES);
        assert!(layout.nodes.iter().all(|n| n.lane < DEFAULT_MAX_LANES));
    }

    #[test]
    fn test_raised_ceiling_allows_more_lanes() {
        let dag = Dag::build(wide_commits());
        let layout = GraphLayout::compute_with_max_lanes(dag, 16);
        assert_eq!(
            layout.lane_count, 12,
            "12 parallel branches fit without recycling under a 16-lane cap"
        );
        // No segment should have been force-recycled.
        assert!(layout.lane_segments.iter().all(|s| !s.recycled));
    }

    #[test]
    fn test_lowered_ceiling_compacts_lanes() {
        let dag = Dag::build(wide_commits());
        let layout = GraphLayout::compute_with_max_lanes(dag, 4);
        assert_eq!(layout.lane_count, 4);
        assert!(layout.nodes.iter().all(|n| n.lane < 4));
    }

    #[test]
    fn test_ceiling_of_default_matches_compute() {
        let layout_a = GraphLayout::compute(Dag::build(wide_commits()));
        let layout_b =
            GraphLayout::compute_with_max_lanes(Dag::build(wide_commits()), DEFAULT_MAX_LANES);
        assert_eq!(layout_a.lane_count, layout_b.lane_count);
        let lanes_a: Vec<usize> = layout_a.nodes.iter().map(|n| n.lane).collect();
        let lanes_b: Vec<usize> = layout_b.nodes.iter().map(|n| n.lane).collect();
        assert_eq!(lanes_a, lanes_b, "compute must equal explicit default cap");
    }

    #[test]
    fn test_head_lane_detected() {
        let commits = vec![
            make_commit("a", &[], &["HEAD", "refs/heads/main"], "latest"),
            make_commit("b", &["a"], &[], "previous"),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        assert!(layout.head_lane.is_some());
        let head_node = layout.nodes.iter().find(|n| n.oid == "a").unwrap();
        assert_eq!(layout.head_lane.unwrap(), head_node.lane);
    }

    #[test]
    fn test_head_lane_none_when_no_head() {
        let commits = vec![make_commit("a", &[], &["refs/heads/main"], "only")];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        assert!(layout.head_lane.is_none());
    }

    #[test]
    fn test_sync_state_no_remotes() {
        let commits = vec![
            make_commit("c", &["b"], &["refs/heads/main"], "tip"),
            commit("b", &["a"]),
            commit("a", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        for seg in &layout.lane_segments {
            assert_eq!(seg.sync_state, SyncState::Unknown);
        }
    }

    #[test]
    fn test_sync_state_fully_synced() {
        let commits = vec![
            make_commit(
                "c",
                &["b"],
                &["refs/heads/main", "refs/remotes/origin/main"],
                "synced tip",
            ),
            commit("b", &["a"]),
            commit("a", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        for seg in &layout.lane_segments {
            assert_eq!(seg.sync_state, SyncState::Synced);
        }
    }

    #[test]
    fn test_sync_state_local_ahead() {
        let commits = vec![
            make_commit("c", &["b"], &["refs/heads/main"], "local tip"),
            commit("b", &["a"]),
            make_commit("a", &[], &["refs/remotes/origin/main"], "remote tip"),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        assert_eq!(layout.lane_segments.len(), 1);
        let seg = &layout.lane_segments[0];
        assert_eq!(seg.sync_state, SyncState::LocalOnly);
    }

    #[test]
    fn test_sync_state_remote_ahead() {
        let commits = vec![
            make_commit("c", &["b"], &["refs/remotes/origin/main"], "remote tip"),
            make_commit("b", &["a"], &["refs/heads/main"], "local tip"),
            commit("a", &[]),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);
        assert_eq!(layout.lane_segments.len(), 1);
        let seg = &layout.lane_segments[0];
        assert_eq!(seg.sync_state, SyncState::RemoteOnly);
    }

    #[test]
    fn graph_layout_json_round_trip() {
        let original = GraphLayout {
            nodes: vec![LayoutNode {
                oid: "abc123".to_string(),
                lane: 0,
                row: 0,
                refs: vec!["HEAD".to_string(), "refs/heads/main".to_string()],
                summary: "initial".to_string(),
                author: "Alice".to_string(),
                email: "alice@example.com".to_string(),
                timestamp: 1_700_000_000,
                is_merge: false,
                is_root: true,
                segment_group: 0,
            }],
            lane_count: 3,
            lane_segments: vec![LaneSegment {
                lane: 0,
                start_row: 0,
                end_row: 0,
                color_index: 0,
                recycled: false,
                sync_state: SyncState::LocalOnly,
                group_id: 0,
            }],
            merge_curves: vec![MergeCurve {
                from_lane: 1,
                from_row: 0,
                to_lane: 0,
                to_row: 1,
                color_index: 1,
                group_id: 2,
            }],
            head_lane: Some(0),
            viewport_index: OnceLock::new(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: GraphLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(original.nodes.len(), restored.nodes.len());
        assert_eq!(original.lane_count, restored.lane_count);
        assert_eq!(original.head_lane, restored.head_lane);
        assert_eq!(original.lane_segments.len(), restored.lane_segments.len());
        assert_eq!(original.merge_curves.len(), restored.merge_curves.len());
        // Spot-check the field inside each vec.
        assert_eq!(restored.nodes[0].oid, "abc123");
        assert_eq!(restored.nodes[0].refs.len(), 2);
        assert_eq!(restored.lane_segments[0].sync_state, SyncState::LocalOnly);
        assert_eq!(restored.merge_curves[0].from_lane, 1);
    }

    /// Full rebuild of `commits` (newest-first) as the ground truth for the
    /// incremental-prepend assertions.
    fn full(commits: Vec<GraphCommit>) -> GraphLayout {
        GraphLayout::compute(Dag::build(commits))
    }

    #[test]
    fn prepend_single_commit_equals_full_rebuild() {
        // Old layout: b (tip, HEAD+main) → a. Advance: commit c; main+HEAD move
        // to c, b keeps nothing.
        let old = full(vec![
            make_commit("b", &["a"], &["HEAD", "refs/heads/main"], "b"),
            make_commit("a", &[], &[], "a"),
        ]);
        let new_commits = vec![make_commit("c", &["b"], &["HEAD", "refs/heads/main"], "c")];
        let inc = old
            .try_prepend_simple_advance(&new_commits, vec![])
            .expect("simple advance should apply");

        let full_after = full(vec![
            make_commit("c", &["b"], &["HEAD", "refs/heads/main"], "c"),
            make_commit("b", &["a"], &[], "b"),
            make_commit("a", &[], &[], "a"),
        ]);
        assert_eq!(
            structural_diff(&inc, &full_after),
            None,
            "incremental prepend must match a full rebuild"
        );
        assert_eq!(inc.head_lane, Some(0));
    }

    #[test]
    fn prepend_multiple_commits_equals_full_rebuild() {
        let old = full(vec![
            make_commit("t", &["a"], &["HEAD", "refs/heads/main"], "t"),
            make_commit("a", &[], &[], "a"),
        ]);
        // Three new commits, newest-first: z→y→x→t.
        let new_commits = vec![
            make_commit("z", &["y"], &["HEAD", "refs/heads/main"], "z"),
            commit("y", &["x"]),
            commit("x", &["t"]),
        ];
        let inc = old
            .try_prepend_simple_advance(&new_commits, vec![])
            .expect("simple advance should apply");

        let full_after = full(vec![
            make_commit("z", &["y"], &["HEAD", "refs/heads/main"], "z"),
            commit("y", &["x"]),
            commit("x", &["t"]),
            make_commit("t", &["a"], &[], "t"),
            make_commit("a", &[], &[], "a"),
        ]);
        assert_eq!(structural_diff(&inc, &full_after), None);
        assert_eq!(inc.nodes.len(), 5);
        assert_eq!(inc.nodes[0].oid, "z");
        assert_eq!(inc.nodes[0].row, 0);
    }

    #[test]
    fn prepend_over_merge_heavy_tail_equals_full_rebuild() {
        // Recycling-heavy tail (mirrors test_lanes_are_recycled) with `main`
        // on the merge tip m2. Advance: commit `top`; main moves to it.
        let tail = || {
            vec![
                make_commit("m2", &["m1", "f2"], &["refs/heads/main"], "merge f2"),
                make_commit("f2", &["base"], &["feature2"], "f2 work"),
                make_commit("m1", &["base", "f1"], &[], "merge f1"),
                make_commit("f1", &["base"], &["feature1"], "f1 work"),
                make_commit("base", &[], &[], "initial"),
            ]
        };
        let old = full(tail());
        let new_commits = vec![make_commit("top", &["m2"], &["refs/heads/main"], "top")];
        let inc = old
            .try_prepend_simple_advance(&new_commits, vec![])
            .expect("simple advance should apply over a merge-heavy tail");

        let mut after = tail();
        after[0].refs = vec![]; // main moved off m2
        after.insert(0, make_commit("top", &["m2"], &["refs/heads/main"], "top"));
        let full_after = full(after);
        assert_eq!(structural_diff(&inc, &full_after), None);
    }

    #[test]
    fn prepend_rejects_off_shape_inputs() {
        let old = full(vec![commit("b", &["a"]), commit("a", &[])]);

        // Empty new set.
        assert!(old.try_prepend_simple_advance(&[], vec![]).is_none());
        // A merge in the new range (two parents).
        assert!(
            old.try_prepend_simple_advance(&[commit("m", &["b", "x"])], vec![])
                .is_none()
        );
        // Chain doesn't attach to the current tip (parent is not "b").
        assert!(
            old.try_prepend_simple_advance(&[commit("c", &["zzz"])], vec![])
                .is_none()
        );
        // Broken chain between two new commits.
        assert!(
            old.try_prepend_simple_advance(&[commit("z", &["y"]), commit("w", &["b"])], vec![])
                .is_none()
        );
    }

    #[test]
    fn test_node_segment_group_matches_covering_segment() {
        // Invariant: every node's `segment_group` equals the `group_id`
        // of the lane segment that covers its (lane, row). If a recycled
        // lane ever bumped the group out of step between the node and its
        // segment, hit-testing would select a different branch when you
        // click the line vs. the dot. Exercise the recycling-heavy shape
        // from `test_lanes_are_recycled` plus the affinity fixtures.
        let commits = vec![
            make_commit("m2", &["m1", "f2"], &["main"], "merge f2"),
            make_commit("f2", &["base"], &["feature2"], "f2 work"),
            make_commit("m1", &["base", "f1"], &[], "merge f1"),
            make_commit("f1", &["base"], &["feature1"], "f1 work"),
            make_commit("base", &[], &[], "initial"),
        ];
        let dag = Dag::build(commits);
        let layout = GraphLayout::compute(dag);

        for node in &layout.nodes {
            // The segment on the node's lane whose row span covers it.
            let covering = layout
                .lane_segments
                .iter()
                .find(|s| s.lane == node.lane && node.row >= s.start_row && node.row <= s.end_row);
            if let Some(seg) = covering {
                assert_eq!(
                    node.segment_group, seg.group_id,
                    "node {} (lane {}, row {}) has segment_group {} but its \
                     covering segment has group_id {}",
                    node.oid, node.lane, node.row, node.segment_group, seg.group_id,
                );
            }
        }
    }
}
