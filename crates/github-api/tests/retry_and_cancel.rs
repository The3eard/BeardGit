//! Integration tests for GitHub retry/cancel endpoints.

use github_api::GitHubProvider;
use provider::CiProvider;

#[tokio::test]
async fn test_retry_run_posts_rerun() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/repos/owner/repo/actions/runs/42/rerun")
        .match_header("Authorization", "Bearer t")
        .with_status(201)
        .create_async()
        .await;
    let p = GitHubProvider::new(&server.url(), "t");
    p.retry_run("owner/repo", "42").await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_retry_failed_jobs_posts_rerun_failed() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock(
            "POST",
            "/repos/owner/repo/actions/runs/42/rerun-failed-jobs",
        )
        .with_status(201)
        .create_async()
        .await;
    let p = GitHubProvider::new(&server.url(), "t");
    p.retry_failed_jobs("owner/repo", "42").await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_retry_job_posts_job_rerun() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/repos/owner/repo/actions/jobs/99/rerun")
        .with_status(201)
        .create_async()
        .await;
    let p = GitHubProvider::new(&server.url(), "t");
    p.retry_job("owner/repo", "99").await.unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn test_cancel_run_posts_cancel() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/repos/owner/repo/actions/runs/42/cancel")
        .with_status(202)
        .create_async()
        .await;
    let p = GitHubProvider::new(&server.url(), "t");
    p.cancel_run("owner/repo", "42").await.unwrap();
    mock.assert_async().await;
}
