//! Integration tests for GitHub `CiProvider::trigger_workflow`.

use github_api::GitHubProvider;
use provider::{CiProvider, TriggerWorkflowInput};

#[tokio::test]
async fn test_trigger_workflow_posts_dispatches() {
    let mut server = mockito::Server::new_async().await;

    // GitHub dispatch returns 204 No Content. Caller must synthesize a run_id —
    // we return a URL-ish placeholder since GitHub doesn't expose the new run ID.
    let dispatch_mock = server
        .mock(
            "POST",
            "/repos/owner/repo/actions/workflows/ci.yml/dispatches",
        )
        .match_header("Authorization", "Bearer test-token")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "ref": "main",
            "inputs": { "deploy_env": "staging" }
        })))
        .with_status(204)
        .create_async()
        .await;

    let provider = GitHubProvider::new(&server.url(), "test-token");
    let mut inputs = std::collections::HashMap::new();
    inputs.insert("deploy_env".to_string(), "staging".to_string());

    let result = provider
        .trigger_workflow(
            "owner/repo",
            &TriggerWorkflowInput {
                workflow_id: "ci.yml".into(),
                git_ref: "main".into(),
                inputs,
            },
        )
        .await
        .unwrap();

    // GitHub's dispatches endpoint does not return the new run_id;
    // the implementation returns an empty string and a reasonable URL.
    assert_eq!(result.run_id, "");
    assert!(result.url.contains("/actions"));
    dispatch_mock.assert_async().await;
}
