//! `get_ci_run_detail` follows job pagination.
//!
//! The endpoint used to be a single `?per_page=100`, so a run with a wide
//! matrix showed the first hundred jobs and said nothing about the rest.
//! GitHub reports `total_count`, so the loop has an exact end condition —
//! these pin that it stops at the right place and doesn't ask for a page it
//! doesn't need.

use github_api::GitHubProvider;
use provider::CiProvider;

/// `n` job objects with sequential ids, as the API would return them.
fn jobs_page(start: u64, n: usize, total: usize) -> String {
    let jobs: Vec<String> = (0..n)
        .map(|i| {
            let id = start + i as u64;
            format!(
                r#"{{"id":{id},"name":"job-{id}","status":"completed","conclusion":"success","started_at":null,"completed_at":null,"html_url":"http://x","steps":[]}}"#
            )
        })
        .collect();
    format!(r#"{{"total_count":{total},"jobs":[{}]}}"#, jobs.join(","))
}

fn run_body() -> &'static str {
    r#"{"id":7,"run_number":1,"name":"CI","status":"completed","conclusion":"success",
        "head_branch":"main","head_sha":"abc","event":"push","created_at":"2026-01-01T00:00:00Z",
        "updated_at":"2026-01-01T00:10:00Z","run_started_at":"2026-01-01T00:00:00Z",
        "html_url":"http://x","actor":null}"#
}

#[tokio::test]
async fn follows_pagination_past_the_first_hundred() {
    let mut server = mockito::Server::new_async().await;
    let run = server
        .mock("GET", "/repos/o/r/actions/runs/7")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(run_body())
        .create_async()
        .await;
    // 250 jobs → three pages: 100, 100, 50.
    let p1 = server
        .mock("GET", "/repos/o/r/actions/runs/7/jobs?per_page=100&page=1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(1, 100, 250))
        .create_async()
        .await;
    let p2 = server
        .mock("GET", "/repos/o/r/actions/runs/7/jobs?per_page=100&page=2")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(101, 100, 250))
        .create_async()
        .await;
    let p3 = server
        .mock("GET", "/repos/o/r/actions/runs/7/jobs?per_page=100&page=3")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(201, 50, 250))
        .create_async()
        .await;

    let provider = GitHubProvider::new(&server.url(), "t");
    let detail = provider.get_ci_run_detail("o/r", 7).await.unwrap();

    assert_eq!(detail.stages.len(), 1, "GitHub has one virtual stage");
    assert_eq!(
        detail.stages[0].jobs.len(),
        250,
        "every job has to survive, not just the first page"
    );
    run.assert_async().await;
    p1.assert_async().await;
    p2.assert_async().await;
    p3.assert_async().await;
}

#[tokio::test]
async fn stops_after_one_page_when_that_is_all_there_is() {
    let mut server = mockito::Server::new_async().await;
    let run = server
        .mock("GET", "/repos/o/r/actions/runs/7")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(run_body())
        .create_async()
        .await;
    let p1 = server
        .mock("GET", "/repos/o/r/actions/runs/7/jobs?per_page=100&page=1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(1, 3, 3))
        .create_async()
        .await;
    // Asking for page 2 with 3 of 3 already in hand would be a wasted
    // round-trip on every pipeline view in the app.
    let p2 = server
        .mock("GET", "/repos/o/r/actions/runs/7/jobs?per_page=100&page=2")
        .with_status(500)
        .expect(0)
        .create_async()
        .await;

    let provider = GitHubProvider::new(&server.url(), "t");
    let detail = provider.get_ci_run_detail("o/r", 7).await.unwrap();

    assert_eq!(detail.stages[0].jobs.len(), 3);
    run.assert_async().await;
    p1.assert_async().await;
    p2.assert_async().await;
}

#[tokio::test]
async fn an_exactly_full_page_does_not_ask_for_another() {
    // The boundary case: 100 of 100. A "page was full, so try the next one"
    // rule would fetch a second, empty page here.
    let mut server = mockito::Server::new_async().await;
    let run = server
        .mock("GET", "/repos/o/r/actions/runs/7")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(run_body())
        .create_async()
        .await;
    let p1 = server
        .mock("GET", "/repos/o/r/actions/runs/7/jobs?per_page=100&page=1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(jobs_page(1, 100, 100))
        .create_async()
        .await;
    let p2 = server
        .mock("GET", "/repos/o/r/actions/runs/7/jobs?per_page=100&page=2")
        .with_status(500)
        .expect(0)
        .create_async()
        .await;

    let provider = GitHubProvider::new(&server.url(), "t");
    let detail = provider.get_ci_run_detail("o/r", 7).await.unwrap();

    assert_eq!(detail.stages[0].jobs.len(), 100);
    run.assert_async().await;
    p1.assert_async().await;
    p2.assert_async().await;
}
