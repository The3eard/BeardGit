//! HTTP executor over `reqwest` with cancellation and body cap.

use std::time::Instant;

use tokio_util::sync::CancellationToken;

use crate::{
    error::RequestsError,
    types::{ExecutionResult, HttpMethod, ResolvedRequest},
};

pub const BODY_CAP_BYTES: usize = 5 * 1024 * 1024;

/// Hard ceiling on how long a single request is allowed to run. Without it
/// a malicious or unreachable host could leave the runner blocked
/// indefinitely, holding cancellation tokens and a tokio task slot.
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub async fn execute(
    req: &ResolvedRequest,
    cancel: CancellationToken,
) -> Result<ExecutionResult, RequestsError> {
    // Reject anything that isn't plain http(s). Variable resolution can
    // produce a URL like `file:///etc/passwd` if a `.http` file references
    // a `{{var}}` that resolves to a local path; rejecting up front keeps
    // the runner from accidentally dereferencing local resources.
    let scheme = req
        .url
        .split_once("://")
        .map(|(s, _)| s.to_ascii_lowercase());
    if !matches!(scheme.as_deref(), Some("http") | Some("https")) {
        return Err(RequestsError::Network(format!(
            "unsupported URL scheme; only http(s) is allowed: {}",
            req.url
        )));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| RequestsError::Network(e.to_string()))?;
    let started = Instant::now();
    let method = match req.method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    };
    let mut builder = client.request(method, &req.url);
    for (k, v) in &req.headers {
        builder = builder.header(k, v);
    }
    if let Some(body) = &req.body {
        builder = builder.body(body.clone());
    }

    let resp = tokio::select! {
        _ = cancel.cancelled() => return Err(RequestsError::Canceled),
        r = builder.send() => r.map_err(|e| RequestsError::Network(e.to_string()))?,
    };

    let status = resp.status().as_u16();
    let headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_bytes = tokio::select! {
        _ = cancel.cancelled() => return Err(RequestsError::Canceled),
        b = resp.bytes() => b.map_err(|e| RequestsError::Network(e.to_string()))?,
    };
    let (body, truncated) = if body_bytes.len() > BODY_CAP_BYTES {
        (body_bytes[..BODY_CAP_BYTES].to_vec(), true)
    } else {
        (body_bytes.to_vec(), false)
    };
    let elapsed = started.elapsed().as_millis() as u64;
    Ok(ExecutionResult {
        status,
        headers,
        body,
        truncated,
        duration_ms: elapsed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn execute_get_200() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("GET", "/x")
            .with_status(200)
            .with_body("hello")
            .create_async()
            .await;

        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: format!("{}/x", server.url()),
            ..Default::default()
        };
        let r = execute(&req, CancellationToken::new()).await.unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.body, b"hello");
        assert!(!r.truncated);
        m.assert_async().await;
    }

    #[tokio::test]
    async fn execute_post_with_body() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("POST", "/u")
            .match_body("{}")
            .with_status(201)
            .with_body("ok")
            .create_async()
            .await;

        let req = ResolvedRequest {
            method: HttpMethod::Post,
            url: format!("{}/u", server.url()),
            body: Some("{}".into()),
            ..Default::default()
        };
        let r = execute(&req, CancellationToken::new()).await.unwrap();
        assert_eq!(r.status, 201);
        m.assert_async().await;
    }

    #[tokio::test]
    async fn cancellation_aborts() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: "https://example.invalid/".into(),
            ..Default::default()
        };
        let err = execute(&req, cancel).await.unwrap_err();
        assert!(matches!(err, RequestsError::Canceled));
    }

    #[tokio::test]
    async fn body_cap_truncates() {
        let mut server = mockito::Server::new_async().await;
        let big = vec![b'A'; BODY_CAP_BYTES + 100];
        server
            .mock("GET", "/big")
            .with_status(200)
            .with_body(big)
            .create_async()
            .await;
        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: format!("{}/big", server.url()),
            ..Default::default()
        };
        let r = execute(&req, CancellationToken::new()).await.unwrap();
        assert_eq!(r.body.len(), BODY_CAP_BYTES);
        assert!(r.truncated);
    }
}
