//! Virtual-scroll viewport over a [`GraphLayout`].
//!
//! The frontend renders the commit graph on a canvas and only needs the rows
//! currently visible on screen. [`GraphLayout::viewport`] slices the full layout
//! to the requested window and returns a [`ViewportResult`] containing nodes,
//! lane segments, and merge curves that overlap the visible area.

use crate::layout::{GraphLayout, LaneSegment, LayoutNode, MergeCurve};
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
    pub fn viewport(&self, offset: usize, limit: usize) -> ViewportResult {
        let total_count = self.nodes.len();
        let start = offset.min(total_count);
        let end = (start + limit).min(total_count);
        let visible = self.nodes[start..end].to_vec();

        let visible_lane_count = visible.iter().map(|n| n.lane + 1).max().unwrap_or(0);

        // Filter lane segments: include if segment overlaps [start, end)
        let lane_segments: Vec<LaneSegment> = self
            .lane_segments
            .iter()
            .filter(|seg| seg.start_row < end && seg.end_row >= start)
            .cloned()
            .collect();

        // Filter merge curves: include if curve's row range overlaps [start, end)
        let merge_curves: Vec<MergeCurve> = self
            .merge_curves
            .iter()
            .filter(|c| {
                let min_row = c.from_row.min(c.to_row);
                let max_row = c.from_row.max(c.to_row);
                min_row < end && max_row >= start
            })
            .cloned()
            .collect();

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
        let layout = GraphLayout::compute(&dag);
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
        let layout = GraphLayout::compute(&dag);
        let result = layout.viewport(45, 20);
        assert_eq!(result.nodes.len(), 5);
        assert_eq!(result.total_count, 50);
        assert_eq!(result.offset, 45);
    }

    #[test]
    fn test_viewport_offset_beyond_end() {
        let dag = linear_dag(10);
        let layout = GraphLayout::compute(&dag);
        let result = layout.viewport(100, 20);
        assert_eq!(result.nodes.len(), 0);
        assert_eq!(result.total_count, 10);
        assert_eq!(result.offset, 10);
    }

    #[test]
    fn test_viewport_includes_overlapping_segments() {
        let dag = linear_dag(100);
        let layout = GraphLayout::compute(&dag);
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
        let layout = GraphLayout::compute(&dag);
        let full = layout.viewport(0, 100);
        let curve_count = full.merge_curves.len();
        let partial = layout.viewport(0, 4);
        assert_eq!(partial.merge_curves.len(), curve_count);
    }

    #[test]
    fn test_viewport_excludes_non_overlapping_segments() {
        let dag = linear_dag(100);
        let layout = GraphLayout::compute(&dag);
        let result = layout.viewport(95, 5);
        assert!(!result.lane_segments.is_empty());
    }
}
