//! Directed acyclic graph (DAG) construction from a flat commit list.
//!
//! Consumers feed a slice of [`GraphCommit`] values (newest-first) into
//! [`Dag::build`], which returns a [`Dag`] with bidirectional parent/child links
//! and derived flags (`is_merge`, `is_root`) ready for the layout stage.

// DAG construction

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Raw commit data provided by the git engine before graph construction.
///
/// The `parents` list follows git conventions: first parent is the mainline,
/// additional parents indicate a merge commit.
#[derive(Debug, Clone)]
pub struct GraphCommit {
    /// Full SHA-1 object ID of the commit.
    pub oid: String,
    /// OIDs of this commit's parents (empty for root commits).
    pub parents: Vec<String>,
    /// Unix timestamp of the author date, used for ordering.
    pub timestamp: i64,
    /// Branch and tag names that point at this commit (e.g. `"HEAD"`, `"main"`).
    pub refs: Vec<String>,
    /// First line of the commit message.
    pub summary: String,
    /// Author display name.
    pub author: String,
    /// Author email address.
    pub email: String,
}

/// A single node in the constructed DAG, enriched with child links and flags.
///
/// Serialized and sent to the frontend as part of the graph layout payload.
#[derive(Debug, Clone, Serialize)]
pub struct DagNode {
    /// Full SHA-1 object ID of the commit.
    pub oid: Arc<str>,
    /// OIDs of this node's parents (empty for root commits).
    pub parents: Vec<Arc<str>>,
    /// OIDs of commits that have this node as a parent.
    pub children: Vec<Arc<str>>,
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
    /// Position of this node in the original insertion order (0 = newest).
    pub index: usize,
    /// `true` when the commit has more than one parent.
    pub is_merge: bool,
    /// `true` when the commit has no parents (repository root or orphan branch).
    pub is_root: bool,
}

/// The fully constructed commit DAG.
///
/// Preserves the insertion order of the input commits while also providing
/// O(1) lookup by OID via an internal hash map.
#[derive(Debug, Default)]
pub struct Dag {
    // Ordered list of node oids (insertion order from commits)
    order: Vec<Arc<str>>,
    // Map from oid to node
    nodes: HashMap<Arc<str>, DagNode>,
}

impl Dag {
    /// Build a DAG from an owned vector of commits.
    ///
    /// Takes the commits by value so the per-commit `String` / `Vec<String>`
    /// fields (`refs`, `summary`, `author`, `email`) can be moved into the
    /// `DagNode` rather than cloned. For repos with hundreds of thousands
    /// of commits the avoided allocations add up — the previous slice-based
    /// API cloned five owned fields per commit on every layout rebuild.
    ///
    /// Two-pass:
    /// 1. Create a `DagNode` for every commit.
    /// 2. Iterate nodes and, for each parent reference, push the current node's
    ///    oid into the parent's `children` list.
    pub fn build(commits: Vec<GraphCommit>) -> Self {
        let mut dag = Dag::default();
        dag.order.reserve(commits.len());
        dag.nodes.reserve(commits.len());

        // Pass 1 — create nodes
        for (index, commit) in commits.into_iter().enumerate() {
            let oid: Arc<str> = commit.oid.as_str().into();
            let parents: Vec<Arc<str>> = commit
                .parents
                .into_iter()
                .map(|s| Arc::from(s.as_str()))
                .collect();
            let is_merge = parents.len() > 1;
            let is_root = parents.is_empty();
            let node = DagNode {
                oid: Arc::clone(&oid),
                parents,
                children: Vec::new(),
                refs: commit.refs,
                summary: commit.summary,
                author: commit.author,
                email: commit.email,
                timestamp: commit.timestamp,
                index,
                is_merge,
                is_root,
            };
            dag.order.push(Arc::clone(&oid));
            dag.nodes.insert(oid, node);
        }

        // Pass 2 — populate children
        // Collect edges first to avoid borrow-checker issues
        let edges: Vec<(Arc<str>, Arc<str>)> = dag
            .order
            .iter()
            .flat_map(|oid| {
                let node = &dag.nodes[oid.as_ref()];
                node.parents
                    .iter()
                    .map(|parent_oid| (Arc::clone(parent_oid), Arc::clone(oid)))
                    .collect::<Vec<_>>()
            })
            .collect();

        for (parent_oid, child_oid) in edges {
            if let Some(parent_node) = dag.nodes.get_mut(parent_oid.as_ref()) {
                parent_node.children.push(child_oid);
            }
        }

        dag
    }

    /// Ordered slice of all nodes.
    pub fn nodes(&self) -> Vec<&DagNode> {
        self.order
            .iter()
            .map(|oid| &self.nodes[oid.as_ref()])
            .collect()
    }

    /// Returns the number of nodes in the DAG.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Returns `true` if the DAG contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Look up a node by its OID, returning `None` if it is not in the DAG.
    pub fn get(&self, oid: &str) -> Option<&DagNode> {
        self.nodes.get(oid)
    }

    /// Consume the DAG and return its nodes in insertion order, transferring
    /// ownership of every node's owned fields (`refs`, `summary`, `author`,
    /// `email`). Used by `GraphLayout::compute` to avoid cloning ~5 owned
    /// fields per commit on every layout rebuild.
    ///
    /// Returns the parent map alongside so the caller can still answer
    /// "who are the parents of this oid?" after the DAG has been consumed.
    pub fn into_ordered_nodes_with_parents(self) -> OrderedDagNodes {
        let Dag { order, mut nodes } = self;
        let mut parents_map: ParentMap = HashMap::with_capacity(order.len());
        let mut ordered: Vec<DagNode> = Vec::with_capacity(order.len());
        for oid in order {
            if let Some(node) = nodes.remove(&oid) {
                parents_map.insert(Arc::clone(&oid), node.parents.clone());
                ordered.push(node);
            }
        }
        (ordered, parents_map)
    }
}

/// Map of commit OID → its parents' OIDs. Used by [`GraphLayout::compute`]
/// after consuming the DAG, so it can still answer parent lookups.
pub type ParentMap = HashMap<Arc<str>, Vec<Arc<str>>>;

/// Tuple returned by [`Dag::into_ordered_nodes_with_parents`]: the nodes in
/// insertion order plus a parent-only view of the original DAG.
pub type OrderedDagNodes = (Vec<DagNode>, ParentMap);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn arc_strs(v: &[Arc<str>]) -> Vec<&str> {
        v.iter().map(|s| s.as_ref()).collect()
    }

    fn commit(oid: &str, parents: &[&str], refs: &[&str]) -> GraphCommit {
        GraphCommit {
            oid: oid.to_string(),
            parents: parents.iter().map(|s| s.to_string()).collect(),
            timestamp: 0,
            refs: refs.iter().map(|s| s.to_string()).collect(),
            summary: format!("commit {}", oid),
            author: String::new(),
            email: String::new(),
        }
    }

    #[test]
    fn test_linear_history() {
        // c → b → a  (newest first)
        let commits = vec![
            commit("c", &["b"], &["HEAD", "main"]),
            commit("b", &["a"], &[]),
            commit("a", &[], &[]),
        ];
        let dag = Dag::build(commits);

        assert_eq!(dag.len(), 3);

        let a = dag.get("a").unwrap();
        assert!(a.is_root);
        assert!(!a.is_merge);
        assert_eq!(arc_strs(&a.children), vec!["b"]);

        let c = dag.get("c").unwrap();
        assert!(c.refs.contains(&"HEAD".to_string()));
        assert!(c.refs.contains(&"main".to_string()));
        assert_eq!(arc_strs(&c.parents), vec!["b"]);
        assert!(c.children.is_empty());
    }

    #[test]
    fn test_merge_commit() {
        // m is a merge of b1 and b2
        let commits = vec![
            commit("m", &["b1", "b2"], &[]),
            commit("b1", &["base"], &[]),
            commit("b2", &["base"], &[]),
            commit("base", &[], &[]),
        ];
        let dag = Dag::build(commits);

        let m = dag.get("m").unwrap();
        assert!(m.is_merge);
        assert_eq!(m.parents.len(), 2);
    }

    #[test]
    fn test_empty_input() {
        let dag = Dag::build(vec![]);
        assert!(dag.is_empty());
        assert_eq!(dag.len(), 0);
        assert!(dag.nodes().is_empty());
    }

    #[test]
    fn test_children_populated() {
        let commits = vec![commit("b", &["a"], &[]), commit("a", &[], &[])];
        let dag = Dag::build(commits);

        let a = dag.get("a").unwrap();
        assert_eq!(arc_strs(&a.children), vec!["b"], "a should list b as child");

        let b = dag.get("b").unwrap();
        assert!(b.children.is_empty(), "b has no children");
        assert_eq!(arc_strs(&b.parents), vec!["a"]);
    }
}
