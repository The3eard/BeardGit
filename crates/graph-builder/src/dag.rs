//! Directed acyclic graph (DAG) construction from a flat commit list.
//!
//! Consumers feed a slice of [`GraphCommit`] values (newest-first) into
//! [`Dag::build`], which returns a [`Dag`] with bidirectional parent/child links
//! and derived flags (`is_merge`, `is_root`) ready for the layout stage.

// DAG construction

use serde::Serialize;
use std::collections::HashMap;

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
    pub oid: String,
    /// OIDs of this node's parents (empty for root commits).
    pub parents: Vec<String>,
    /// OIDs of commits that have this node as a parent.
    pub children: Vec<String>,
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
    order: Vec<String>,
    // Map from oid to node
    nodes: HashMap<String, DagNode>,
}

impl Dag {
    /// Build a DAG from a slice of commits.
    ///
    /// Two-pass:
    /// 1. Create a `DagNode` for every commit.
    /// 2. Iterate nodes and, for each parent reference, push the current node's
    ///    oid into the parent's `children` list.
    pub fn build(commits: &[GraphCommit]) -> Self {
        let mut dag = Dag::default();

        // Pass 1 — create nodes
        for (index, commit) in commits.iter().enumerate() {
            let node = DagNode {
                oid: commit.oid.clone(),
                parents: commit.parents.clone(),
                children: Vec::new(),
                refs: commit.refs.clone(),
                summary: commit.summary.clone(),
                author: commit.author.clone(),
                email: commit.email.clone(),
                timestamp: commit.timestamp,
                index,
                is_merge: commit.parents.len() > 1,
                is_root: commit.parents.is_empty(),
            };
            dag.order.push(commit.oid.clone());
            dag.nodes.insert(commit.oid.clone(), node);
        }

        // Pass 2 — populate children
        // Collect edges first to avoid borrow-checker issues
        let edges: Vec<(String, String)> = dag
            .order
            .iter()
            .flat_map(|oid| {
                let node = &dag.nodes[oid];
                node.parents
                    .iter()
                    .map(|parent_oid| (parent_oid.clone(), oid.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        for (parent_oid, child_oid) in edges {
            if let Some(parent_node) = dag.nodes.get_mut(&parent_oid) {
                parent_node.children.push(child_oid);
            }
        }

        dag
    }

    /// Ordered slice of all nodes.
    pub fn nodes(&self) -> Vec<&DagNode> {
        self.order.iter().map(|oid| &self.nodes[oid]).collect()
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
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

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
        let dag = Dag::build(&commits);

        assert_eq!(dag.len(), 3);

        let a = dag.get("a").unwrap();
        assert!(a.is_root);
        assert!(!a.is_merge);
        assert_eq!(a.children, vec!["b"]);

        let c = dag.get("c").unwrap();
        assert!(c.refs.contains(&"HEAD".to_string()));
        assert!(c.refs.contains(&"main".to_string()));
        assert_eq!(c.parents, vec!["b"]);
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
        let dag = Dag::build(&commits);

        let m = dag.get("m").unwrap();
        assert!(m.is_merge);
        assert_eq!(m.parents.len(), 2);
    }

    #[test]
    fn test_empty_input() {
        let dag = Dag::build(&[]);
        assert!(dag.is_empty());
        assert_eq!(dag.len(), 0);
        assert!(dag.nodes().is_empty());
    }

    #[test]
    fn test_children_populated() {
        let commits = vec![commit("b", &["a"], &[]), commit("a", &[], &[])];
        let dag = Dag::build(&commits);

        let a = dag.get("a").unwrap();
        assert_eq!(a.children, vec!["b"], "a should list b as child");

        let b = dag.get("b").unwrap();
        assert!(b.children.is_empty(), "b has no children");
        assert_eq!(b.parents, vec!["a"]);
    }
}
