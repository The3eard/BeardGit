//! Integration tests for GitLab retry/cancel endpoints against a mock HTTP server.

use gitlab_api::GitLabProvider;
use provider::CiProvider;

#[tokio::test]
async fn test_retry_run_posts_retry() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock(
            "POST",
            "/api/v4/projects/group%2Fproject/pipelines/42/retry",
        )
        .match_header("PRIVATE-TOKEN", "t")
        .with_status(201)
        .with_body(r#"{"id":42}"#)
        .create_async()
        .await;
    let p = GitLabProvider::new(&server.url(), "t");
    p.retry_run("group/project", "42").await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_retry_failed_jobs_uses_same_endpoint() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock(
            "POST",
            "/api/v4/projects/group%2Fproject/pipelines/42/retry",
        )
        .with_status(201)
        .with_body(r#"{"id":42}"#)
        .create_async()
        .await;
    let p = GitLabProvider::new(&server.url(), "t");
    p.retry_failed_jobs("group/project", "42").await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_retry_job_posts_job_retry() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/api/v4/projects/group%2Fproject/jobs/999/retry")
        .with_status(201)
        .with_body(r#"{"id":1000}"#)
        .create_async()
        .await;
    let p = GitLabProvider::new(&server.url(), "t");
    p.retry_job("group/project", "999").await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_cancel_run_posts_cancel() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock(
            "POST",
            "/api/v4/projects/group%2Fproject/pipelines/42/cancel",
        )
        .with_status(200)
        .with_body(r#"{"id":42}"#)
        .create_async()
        .await;
    let p = GitLabProvider::new(&server.url(), "t");
    p.cancel_run("group/project", "42").await.unwrap();
    mock.assert_async().await;
}
