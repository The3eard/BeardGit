//! Extended CI provider types introduced in Phase 8.4 (CI/CD control).
//!
//! Kept in a submodule so the core `CiProvider` trait + types remain in
//! `lib.rs`. All types are re-exported from the crate root.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input for triggering a new CI run.
///
/// For GitHub Actions, `workflow_id` must be the workflow file name
/// (e.g. `"ci.yml"`) or the numeric workflow ID. For GitLab, `workflow_id`
/// is ignored — there is a single `.gitlab-ci.yml` per project and the
/// trigger is parameterized by `git_ref` only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerWorkflowInput {
    /// GitHub: workflow file name or numeric ID. GitLab: ignored.
    pub workflow_id: String,
    /// Branch or tag name to run against.
    pub git_ref: String,
    /// Workflow-dispatch inputs (GitHub) or pipeline variables (GitLab).
    pub inputs: HashMap<String, String>,
}

/// Result of a successful trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerResult {
    /// Provider-specific run identifier (returned as a string so callers
    /// don't have to know whether GitHub `run_id` is `u64` or `u128`).
    pub run_id: String,
    /// URL to the run in the provider's web UI.
    pub url: String,
}

/// Workflow definition metadata (GitHub only — GitLab returns a placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Provider-specific ID (GitHub: numeric workflow ID as string;
    /// GitLab: always `"default"`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Path to the workflow file inside the repository.
    pub path: String,
    /// Enablement state.
    pub state: WorkflowState,
}

/// Enablement state for a workflow definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowState {
    /// Workflow is active and can be triggered.
    Active,
    /// Workflow is disabled by the repo admin (GitHub).
    Disabled,
}
