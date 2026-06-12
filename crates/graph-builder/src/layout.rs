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

use crate::dag::Dag;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
        }
    }
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
}
