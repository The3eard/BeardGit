//! Graph builder for commit DAG visualization.
//!
//! This crate takes a flat list of commits and produces a fully laid-out graph
//! that can be rendered by the frontend. The pipeline has three stages:
//!
//! 1. **DAG** (`dag` module) — builds a directed acyclic graph from raw commits.
//! 2. **Layout** (`layout` module) — assigns each node a row and lane, computes
//!    lane segments (continuous vertical lines) and merge curves (cross-lane edges).
//! 3. **Viewport** (`viewport` module) — slices the layout for virtual scrolling.

pub mod dag;
pub mod layout;
pub mod viewport;

pub use dag::{Dag, DagNode, GraphCommit};
pub use layout::{GraphLayout, LaneSegment, LayoutNode, MergeCurve, SyncState};
pub use viewport::ViewportResult;
