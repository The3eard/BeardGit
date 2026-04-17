//! Integration tests for GitHub `CiProvider::list_workflows`.

use github_api::GitHubProvider;
use provider::{CiProvider, WorkflowState};

#[tokio::test]
async fn test_list_workflows_parses_active_and_disabled() {
    let mut server = mockito::Server::new_async().await;
    let body = r#"{
        "total_count": 2,
        "workflows": [
            {
                "id": 101,
                "name": "CI",
                "path": ".github/workflows/ci.yml",
                "state": "active"
            },
            {
                "id": 202,
                "name": "Deploy",
                "path": ".github/workflows/deploy.yml",
                "state": "disabled_manually"
            }
        ]
    }"#;
    let mock = server
        .mock("GET", "/repos/owner/repo/actions/workflows")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;
    let p = GitHubProvider::new(&server.url(), "t");
    let workflows = p.list_workflows("owner/repo").await.unwrap();
    assert_eq!(workflows.len(), 2);
    assert_eq!(workflows[0].id, "101");
    assert_eq!(workflows[0].name, "CI");
    assert_eq!(workflows[0].state, WorkflowState::Active);
    assert_eq!(workflows[1].state, WorkflowState::Disabled);
    mock.assert_async().await;
}
