//! `list_pipeline_jobs` follows pagination.
//!
//! GitLab returns a bare array with no total, so the end of the list is a
//! short page — a page with fewer entries than `per_page`. That makes the
//! exactly-full-last-page case the interesting one: it costs one extra
//! request, and getting it wrong either drops jobs or loops.
//!
//! The endpoint used to be a single `?per_page=100`, which silently dropped
//! everything past the first hundred.

use gitlab_api::GitLabClient;

/// `n` job objects with sequential ids, as the API would return them.
fn jobs_page(start: u64, n: usize) -> String {
    let jobs: Vec<String> = (0..n)
        .map(|i| {
            let id = start + i as u64;
            format!(
                r#"{{"id":{id},"name":"job-{id}","stage":"test","status":"success","duration":1.0,
                    "created_at":null,"started_at":null,"finished_at":null,
                    "web_url":"http://x","pipeline":null,"allow_failure":false}}"#
            )
        })
        .collect();
    format!("[{}]", jobs.join(","))
}

#[tokio::test]
async fn follows_pagination_past_the_first_hundred() {
    let mut server = mockito::Server::new_async().await;
    let p1 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(1, 100))
        .create_async()
        .await;
    let p2 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(101, 30))
        .create_async()
        .await;

    let client = GitLabClient::new(&server.url(), "t");
    let jobs = client.list_pipeline_jobs(9, 42).await.unwrap();

    assert_eq!(
        jobs.len(),
        130,
        "the short page ends the list, not the first"
    );
    p1.assert_async().await;
    p2.assert_async().await;
}

#[tokio::test]
async fn a_short_first_page_ends_immediately() {
    let mut server = mockito::Server::new_async().await;
    let p1 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(1, 4))
        .create_async()
        .await;
    let p2 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=2",
        )
        .with_status(500)
        .expect(0)
        .create_async()
        .await;

    let client = GitLabClient::new(&server.url(), "t");
    let jobs = client.list_pipeline_jobs(9, 42).await.unwrap();

    assert_eq!(jobs.len(), 4);
    p1.assert_async().await;
    p2.assert_async().await;
}

#[tokio::test]
async fn an_exactly_full_last_page_costs_one_empty_request() {
    // Without a total, a full page is indistinguishable from "there is more",
    // so 100 jobs means asking for page 2 and getting nothing. Pinned so the
    // behaviour is a choice rather than a surprise.
    let mut server = mockito::Server::new_async().await;
    let p1 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(1, 100))
        .create_async()
        .await;
    let p2 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let client = GitLabClient::new(&server.url(), "t");
    let jobs = client.list_pipeline_jobs(9, 42).await.unwrap();

    assert_eq!(jobs.len(), 100);
    p1.assert_async().await;
    p2.assert_async().await;
}

#[tokio::test]
async fn stage_grouping_sees_every_page() {
    // `get_pipeline_stages` groups by stage on top of the paginated fetch, so
    // a stage that only appears on page 2 has to show up.
    let mut server = mockito::Server::new_async().await;
    let page2 = r#"[{"id":200,"name":"deploy-1","stage":"deploy","status":"success","duration":1.0,
        "created_at":null,"started_at":null,"finished_at":null,"web_url":"http://x",
        "pipeline":null,"allow_failure":false}]"#;
    let p1 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(1, 100))
        .create_async()
        .await;
    let p2 = server
        .mock(
            "GET",
            "/api/v4/projects/9/pipelines/42/jobs?per_page=100&page=2",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page2)
        .create_async()
        .await;

    let client = GitLabClient::new(&server.url(), "t");
    let stages = client.get_pipeline_stages(9, 42).await.unwrap();

    let names: Vec<&str> = stages.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"deploy"),
        "a stage introduced on page 2 must not be lost: got {names:?}"
    );
    p1.assert_async().await;
    p2.assert_async().await;
}
