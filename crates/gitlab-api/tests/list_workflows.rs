//! Integration tests for GitLab `CiProvider::list_workflows`.

use gitlab_api::GitLabProvider;
use provider::{CiProvider, WorkflowState};

#[tokio::test]
async fn test_list_workflows_returns_single_placeholder() {
    // No mockito needed — GitLab `list_workflows` is synthetic.
    let p = GitLabProvider::new("https://gitlab.example.com", "t");
    let workflows = p.list_workflows("group/project").await.unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0].id, "default");
    assert_eq!(workflows[0].path, ".gitlab-ci.yml");
    assert_eq!(workflows[0].state, WorkflowState::Active);
}
