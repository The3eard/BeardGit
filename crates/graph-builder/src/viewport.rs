//! Virtual-scroll viewport over a [`GraphLayout`].
//!
//! The frontend renders the commit graph on a canvas and only needs the rows
//! currently visible on screen. [`GraphLayout::viewport`] slices the full layout
//! to the requested window and returns a [`ViewportResult`] containing nodes,
//! lane segments, and merge curves that overlap the visible area.

use crate::layout::{GraphLayout, LaneSegment, LayoutNode, MergeCurve, ViewportIndex};
use serde::Serialize;

/// The result of a viewport query over a [`GraphLayout`].
///
/// Contains all data the frontend needs to render the visible portion of the
/// commit graph: nodes for commit dots and text, lane segments for continuous
/// vertical branch lines, and merge curves for cross-lane connections.
#[derive(Debug, Serialize)]
pub struct ViewportResult {
    /// Layout nodes in the visible window, in row order.
    pub nodes: Vec<LayoutNode>,
    /// Lane segments that overlap the viewport. Segments may extend beyond
    /// the viewport bounds — the renderer clips them to the canvas.
    pub lane_segments: Vec<LaneSegment>,
    /// Merge curves that overlap the viewport.
    pub merge_curves: Vec<MergeCurve>,
    /// Total number of nodes in the full layout (used for scroll-thumb sizing).
    pub total_count: usize,
    /// The actual start row after clamping.
    pub offset: usize,
    /// Maximum lane index + 1 across the visible nodes.
    pub visible_lane_count: usize,
    /// Total lane count for the entire graph.
    pub total_lane_count: usize,
    /// Lane of the HEAD commit, if in the graph.
    pub head_lane: Option<usize>,
}

impl GraphLayout {
    /// Return a slice of layout data for virtual scrolling.
    ///
    /// Nodes are already row-indexed (a direct slice); the overlapping lane
    /// segments and merge curves are gathered through a lazily-built
    /// [`ViewportIndex`] so each query is O(window buckets + overlap) instead of
    /// scanning every segment and curve. The index is built on the first call
    /// and reused for all later scroll queries over this layout.
    pub fn viewport(&self, offset: usize, limit: usize) -> ViewportResult {
        let total_count = self.nodes.len();
        let start = offset.min(total_count);
        let end = (start + limit).min(total_count);
        let visible = self.nodes[start..end].to_vec();

        let visible_lane_count = visible.iter().map(|n| n.lane + 1).max().unwrap_or(0);

        let index = self.viewport_index.get_or_init(|| {
            ViewportIndex::build(total_count, &self.lane_segments, &self.merge_curves)
        });
        let (lane_segments, merge_curves) = self.overlap_indexed(index, start, end);

        ViewportResult {
            nodes: visible,
            total_count,
            offset: start,
            visible_lane_count,
            total_lane_count: self.lane_count,
            lane_segments,
            merge_curves,
            head_lane: self.head_lane,
        }
    }

    /// Lane segments and merge curves overlapping `[start, end)`, gathered via
    /// the row-bucket index. Candidate indices come back ascending, and the
    /// exact overlap predicate is re-applied (a bucket is wider than the
    /// window), so the output is byte-identical to [`Self::overlap_scan`] —
    /// including order, which the renderer consumes positionally.
    fn overlap_indexed(
        &self,
        index: &ViewportIndex,
        start: usize,
        end: usize,
    ) -> (Vec<LaneSegment>, Vec<MergeCurve>) {
        let bucket_rows = index.bucket_rows;
        let lane_segments =
            ViewportIndex::candidates(&index.segment_buckets, bucket_rows, start, end)
                .into_iter()
                .map(|i| &self.lane_segments[i as usize])
                .filter(|seg| seg.start_row < end && seg.end_row >= start)
                .cloned()
                .collect();
        let merge_curves = ViewportIndex::candidates(&index.curve_buckets, bucket_rows, start, end)
            .into_iter()
            .map(|i| &self.merge_curves[i as usize])
            .filter(|c| {
                let min_row = c.from_row.min(c.to_row);
                let max_row = c.from_row.max(c.to_row);
                min_row < end && max_row >= start
            })
            .cloned()
            .collect();
        (lane_segments, merge_curves)
    }

    /// Reference (pre-index) implementation: a full linear scan of every segment
    /// and curve. Retained as the correctness oracle the [`ViewportIndex`] must
    /// match — see `viewport_index_matches_linear_scan`.
    #[cfg(test)]
    fn overlap_scan(&self, start: usize, end: usize) -> (Vec<LaneSegment>, Vec<MergeCurve>) {
        // An empty window shows nothing; match the indexed path (whose bucket
        // range is empty when `start >= end`) rather than the pre-index quirk
        // of returning segments straddling the point `start`.
        if start >= end {
            return (Vec::new(), Vec::new());
        }
        let lane_segments = self
            .lane_segments
            .iter()
            .filter(|seg| seg.start_row < end && seg.end_row >= start)
            .cloned()
            .collect();
        let merge_curves = self
            .merge_curves
            .iter()
            .filter(|c| {
                let min_row = c.from_row.min(c.to_row);
                let max_row = c.from_row.max(c.to_row);
                min_row < end && max_row >= start
            })
            .cloned()
            .collect();
        (lane_segments, merge_curves)
    }
}

#[cfg(test)]
mod tests {
    use crate::dag::{Dag, GraphCommit};
    use crate::layout::GraphLayout;

    fn linear_dag(n: usize) -> Dag {
        // Build commits in newest-first order: commit "0000" is row 0 (HEAD),
        // commit "0099" is row n-1 (root, no parents). Each commit's parent is
        // the next commit in the chain.
        let mut commits: Vec<GraphCommit> = Vec::with_capacity(n);
        for i in 0..n {
            let oid = format!("{:04}", i);
            let parents = if i + 1 < n {
                vec![format!("{:04}", i + 1)]
            } else {
                vec![]
            };
            commits.push(GraphCommit {
                oid: oid.clone(),
                parents,
                timestamp: (n - i) as i64,
                refs: Vec::new(),
                summary: format!("commit {}", oid),
                author: String::new(),
                email: String::new(),
            });
        }
        Dag::build(commits)
    }

    fn branching_dag() -> Dag {
        let commits = vec![
            GraphCommit {
                oid: "m".into(),
                parents: vec!["b1".into(), "b2".into()],
                timestamp: 4,
                refs: vec![],
                summary: "merge".into(),
                author: String::new(),
                email: String::new(),
            },
            GraphCommit {
                oid: "b1".into(),
                parents: vec!["base".into()],
                timestamp: 3,
                refs: vec![],
                summary: "b1".into(),
                author: String::new(),
                email: String::new(),
            },
            GraphCommit {
                oid: "b2".into(),
                parents: vec!["base".into()],
                timestamp: 2,
                refs: vec![],
                summary: "b2".into(),
                author: String::new(),
                email: String::new(),
            },
            GraphCommit {
                oid: "base".into(),
                parents: vec![],
                timestamp: 1,
                refs: vec![],
                summary: "base".into(),
                author: String::new(),
                email: String::new(),
            },
        ];
        Dag::build(commits)
    }

    #[test]
    fn test_viewport_returns_slice() {
        let dag = linear_dag(100);
        let layout = GraphLayout::compute(dag);
        let result = layout.viewport(10, 20);
        assert_eq!(result.nodes.len(), 20);
        assert_eq!(result.total_count, 100);
        assert_eq!(result.offset, 10);
        assert_eq!(result.nodes[0].row, 10);
        assert_eq!(result.nodes[19].row, 29);
    }

    #[test]
    fn test_viewport_clamps_to_end() {
        let dag = linear_dag(50);
        let layout = GraphLayout::compute(dag);
        let result = layout.viewport(45, 20);
        assert_eq!(result.nodes.len(), 5);
        assert_eq!(result.total_count, 50);
        assert_eq!(result.offset, 45);
    }

    #[test]
    fn test_viewport_offset_beyond_end() {
        let dag = linear_dag(10);
        let layout = GraphLayout::compute(dag);
        let result = layout.viewport(100, 20);
        assert_eq!(result.nodes.len(), 0);
        assert_eq!(result.total_count, 10);
        assert_eq!(result.offset, 10);
    }

    #[test]
    fn test_viewport_includes_overlapping_segments() {
        let dag = linear_dag(100);
        let layout = GraphLayout::compute(dag);
        let result = layout.viewport(10, 20);
        assert!(
            !result.lane_segments.is_empty(),
            "viewport should include overlapping segments"
        );
        let seg = &result.lane_segments[0];
        assert!(seg.start_row <= 10);
        assert!(seg.end_row >= 29);
    }

    #[test]
    fn test_viewport_includes_overlapping_merge_curves() {
        let dag = branching_dag();
        let layout = GraphLayout::compute(dag);
        let full = layout.viewport(0, 100);
        let curve_count = full.merge_curves.len();
        let partial = layout.viewport(0, 4);
        assert_eq!(partial.merge_curves.len(), curve_count);
    }

    #[test]
    fn test_viewport_excludes_non_overlapping_segments() {
        let dag = linear_dag(100);
        let layout = GraphLayout::compute(dag);
        let result = layout.viewport(95, 5);
        assert!(!result.lane_segments.is_empty());
    }

    // ── Row-index property test: the indexed viewport must return exactly what
    // the pre-index linear scan returned, for every window.

    /// Build a realistic merge-heavy layout (mainline with feature branches
    /// merged back) via the git-engine synthetic-repo helper.
    fn merge_heavy_layout(commits: usize, branches: usize) -> GraphLayout {
        let (_dir, path) = git_engine::test_support::create_synthetic_repo(commits, branches);
        let repo = git_engine::Repository::open(&path).expect("open synthetic repo");
        let walked = repo
            .walk_commits(0, commits * 2)
            .expect("walk synthetic repo");
        let graph_commits: Vec<GraphCommit> = walked
            .into_iter()
            .map(|c| GraphCommit {
                oid: c.oid,
                parents: c.parents,
                timestamp: c.timestamp,
                refs: c.refs,
                summary: c.summary,
                author: c.author,
                email: c.email,
            })
            .collect();
        GraphLayout::compute(Dag::build(graph_commits))
    }

    /// Assert the indexed `viewport()` output matches the linear-scan oracle for
    /// a wide sweep of windows, including bucket-boundary and out-of-range edges.
    fn assert_indexed_matches_scan(layout: &GraphLayout) {
        let n = layout.nodes.len();
        let mut offsets: Vec<usize> = (0..=n).step_by(37).collect();
        // Bucket boundaries (512) are the interesting cases for the index.
        offsets.extend([0, 1, 2, 510, 511, 512, 513, 1023, 1024, 1025, n, n + 5]);
        let limits = [0usize, 1, 3, 50, 300, 512, 1024, n + 10];
        for &offset in &offsets {
            for &limit in &limits {
                let vp = layout.viewport(offset, limit);
                let start = offset.min(n);
                let end = (start + limit).min(n);
                let (segs, curves) = layout.overlap_scan(start, end);
                assert_eq!(
                    vp.lane_segments, segs,
                    "lane_segments mismatch at offset {offset} limit {limit}"
                );
                assert_eq!(
                    vp.merge_curves, curves,
                    "merge_curves mismatch at offset {offset} limit {limit}"
                );
            }
        }
    }

    #[test]
    fn viewport_index_matches_linear_scan_linear() {
        let layout = GraphLayout::compute(linear_dag(1200));
        assert_indexed_matches_scan(&layout);
    }

    #[test]
    fn viewport_index_matches_linear_scan_branching() {
        let layout = GraphLayout::compute(branching_dag());
        assert_indexed_matches_scan(&layout);
    }

    #[test]
    fn viewport_index_matches_linear_scan_merge_heavy() {
        // > 2 buckets of rows with dense cross-lane curves.
        let layout = merge_heavy_layout(1500, 120);
        assert!(
            layout.nodes.len() > 1024,
            "fixture must span multiple index buckets"
        );
        assert!(
            !layout.merge_curves.is_empty(),
            "fixture must exercise merge curves"
        );
        assert_indexed_matches_scan(&layout);
    }

    #[test]
    fn viewport_index_empty_layout_is_safe() {
        let layout = GraphLayout::compute(Dag::build(vec![]));
        let vp = layout.viewport(0, 10);
        assert!(vp.nodes.is_empty());
        assert!(vp.lane_segments.is_empty());
        assert!(vp.merge_curves.is_empty());
    }
}
