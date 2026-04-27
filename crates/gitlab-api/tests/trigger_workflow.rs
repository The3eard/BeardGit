//! Integration tests for GitLab `CiProvider::trigger_workflow` against a mock HTTP server.

use gitlab_api::GitLabProvider;
use provider::{CiProvider, TriggerWorkflowInput};

#[tokio::test]
async fn test_trigger_workflow_posts_pipeline() {
    let mut server = mockito::Server::new_async().await;

    // GitLab: POST /api/v4/projects/{id}/pipeline  — project path is URL-encoded
    // Payload carries the ref and an array of variables.
    let mock = server
        .mock("POST", "/api/v4/projects/group%2Fproject/pipeline")
        .match_header("PRIVATE-TOKEN", "test-token")
        .match_header("content-type", "application/json")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "ref": "main",
            "variables": [
                { "key": "DEPLOY_ENV", "value": "staging" }
            ]
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "id": 12345,
                "web_url": "https://gitlab.com/group/project/-/pipelines/12345"
            }"#,
        )
        .create_async()
        .await;

    let provider = GitLabProvider::new(&server.url(), "test-token");
    let mut inputs = std::collections::HashMap::new();
    inputs.insert("DEPLOY_ENV".to_string(), "staging".to_string());

    let result = provider
        .trigger_workflow(
            "group/project",
            &TriggerWorkflowInput {
                workflow_id: "ignored".to_string(),
                git_ref: "main".to_string(),
                inputs,
            },
        )
        .await
        .unwrap();

    assert_eq!(result.run_id, "12345");
    assert_eq!(
        result.url,
        "https://gitlab.com/group/project/-/pipelines/12345"
    );
    mock.assert_async().await;
}
