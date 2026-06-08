//! HTTP executor over `reqwest` with cancellation and body cap.

use std::net::IpAddr;
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

/// Hostnames that always resolve to the loopback interface or a link-local
/// metadata service. Treated as private regardless of how the user's
/// resolver feels about them.
const FORBIDDEN_HOSTS: &[&str] = &[
    "localhost",
    "ip6-localhost",
    "ip6-loopback",
    "metadata.google.internal",
    "metadata",
    "instance-data", // AWS IMDS DNS alias
];

/// Extract the host portion of a `http(s)://...` URL, lowercased and
/// stripped of any user-info, brackets, and port suffix. IPv6 hosts in
/// URL form are wrapped in `[..]`; we strip the brackets so callers
/// receive the literal address (`::1`, not `[::1]`).
fn extract_host(url: &str) -> Option<String> {
    let after_scheme = url.split_once("://").map(|(_, r)| r)?;
    let authority = after_scheme.split(['/', '?', '#']).next()?;
    let host_with_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if let Some(rest) = host_with_port.strip_prefix('[') {
        // IPv6 literal: keep everything up to the closing bracket.
        rest.split_once(']').map(|(addr, _)| addr).unwrap_or(rest)
    } else {
        host_with_port.split(':').next().unwrap_or(host_with_port)
    };
    Some(host.to_ascii_lowercase())
}

/// Returns `true` for hosts the Requests panel must refuse to dial in
/// the default configuration: loopback, RFC1918, link-local, ULA, and a
/// short list of well-known cloud metadata service aliases.
///
/// The runner used to forward requests to whatever `reqwest` resolved
/// the URL to, which made it trivial for a `.http` file to hit
/// `http://169.254.169.254/latest/meta-data/` (cloud IMDS) or
/// `http://localhost:8080/admin` and exfiltrate the response into the
/// history DB. This screen blocks those targets by default; an opt-in
/// via `BEARDGIT_REQUESTS_ALLOW_PRIVATE=1` is provided for legitimate
/// internal-network use cases.
fn host_is_forbidden(host: &str) -> bool {
    if FORBIDDEN_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(host)) {
        return true;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return ip_is_forbidden(ip);
    }
    false
}

/// Screen a concrete IP against the private/loopback/link-local/ULA/IMDS
/// ranges. Used both for literal-IP URLs and (post-resolution) for hostnames.
fn ip_is_forbidden(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => ipv4_is_forbidden(v4),
        IpAddr::V6(v6) => {
            // An IPv4-mapped address (`::ffff:a.b.c.d`) actually connects to
            // the embedded V4 address, so screen it with the V4 ruleset —
            // otherwise `[::ffff:127.0.0.1]` / `[::ffff:169.254.169.254]`
            // would slip past the V6-only checks below.
            if let Some(v4) = v6.to_ipv4_mapped() {
                return ipv4_is_forbidden(v4);
            }
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // ULA fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
    }
}

fn ipv4_is_forbidden(v4: std::net::Ipv4Addr) -> bool {
    v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_unspecified()
}

/// Optional knobs for [`execute`]. Default reads
/// `BEARDGIT_REQUESTS_ALLOW_PRIVATE` so production callers stay locked
/// down by default; tests pass `allow_private_hosts: true` to talk to
/// `mockito` on `127.0.0.1`.
#[derive(Debug, Clone)]
pub struct ExecuteOptions {
    pub allow_private_hosts: bool,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            allow_private_hosts: matches!(
                std::env::var("BEARDGIT_REQUESTS_ALLOW_PRIVATE").as_deref(),
                Ok("1" | "true" | "yes")
            ),
        }
    }
}

pub async fn execute(
    req: &ResolvedRequest,
    cancel: CancellationToken,
    opts: ExecuteOptions,
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

    let host = extract_host(&req.url).ok_or_else(|| {
        RequestsError::Network(format!("could not parse host from URL: {}", req.url))
    })?;
    if host_is_forbidden(&host) && !opts.allow_private_hosts {
        return Err(RequestsError::Network(format!(
            "host '{host}' targets a private/loopback range; \
             set BEARDGIT_REQUESTS_ALLOW_PRIVATE=1 to permit"
        )));
    }

    // Defense against a public hostname that resolves to a private/loopback/
    // IMDS address (DNS rebinding, or simply an attacker-controlled A record):
    // the string check above only sees the literal host. Resolve it and reject
    // if ANY resolved IP is forbidden. reqwest re-resolves at dial time, so a
    // determined rebinding attacker has a narrow TOCTOU window, but this closes
    // the common case. Skipped for literal IPs (already screened) and when
    // private hosts are explicitly allowed.
    if !opts.allow_private_hosts
        && host.parse::<IpAddr>().is_err()
        && let Ok(addrs) = tokio::net::lookup_host(format!("{host}:0")).await
    {
        for addr in addrs {
            if ip_is_forbidden(addr.ip()) {
                return Err(RequestsError::Network(format!(
                    "host '{host}' resolves to a private/loopback address ({}); \
                     set BEARDGIT_REQUESTS_ALLOW_PRIVATE=1 to permit",
                    addr.ip()
                )));
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        // Don't follow redirects automatically — a 302 from a public host
        // to `http://169.254.169.254/...` would otherwise re-emit the
        // user's `Authorization` header against a target we don't trust.
        // The `.http` panel surfaces redirects in the response so the
        // user can re-issue if they're expected.
        .redirect(reqwest::redirect::Policy::none())
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
        let r = execute(
            &req,
            CancellationToken::new(),
            ExecuteOptions {
                allow_private_hosts: true,
            },
        )
        .await
        .unwrap();
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
        let r = execute(
            &req,
            CancellationToken::new(),
            ExecuteOptions {
                allow_private_hosts: true,
            },
        )
        .await
        .unwrap();
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
        let err = execute(&req, cancel, ExecuteOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, RequestsError::Canceled));
    }

    #[tokio::test]
    async fn rejects_loopback_host() {
        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: "http://localhost:8080/admin".into(),
            ..Default::default()
        };
        let err = execute(&req, CancellationToken::new(), ExecuteOptions::default())
            .await
            .unwrap_err();
        assert!(
            matches!(err, RequestsError::Network(ref m) if m.contains("private/loopback")),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn rejects_imds_metadata_host() {
        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: "http://169.254.169.254/latest/meta-data/".into(),
            ..Default::default()
        };
        let err = execute(&req, CancellationToken::new(), ExecuteOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, RequestsError::Network(_)));
    }

    #[tokio::test]
    async fn rejects_rfc1918_v4_host() {
        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: "http://192.168.1.1/admin".into(),
            ..Default::default()
        };
        let err = execute(&req, CancellationToken::new(), ExecuteOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, RequestsError::Network(_)));
    }

    #[tokio::test]
    async fn rejects_ipv6_loopback_host() {
        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: "http://[::1]/admin".into(),
            ..Default::default()
        };
        let err = execute(&req, CancellationToken::new(), ExecuteOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, RequestsError::Network(_)));
    }

    #[test]
    fn extract_host_strips_userinfo_and_port() {
        assert_eq!(
            extract_host("https://user:pass@example.com:8443/x"),
            Some("example.com".into())
        );
    }

    #[test]
    fn extract_host_handles_ipv6_brackets() {
        assert_eq!(extract_host("http://[::1]:8080/").as_deref(), Some("::1"));
    }

    #[test]
    fn ipv4_mapped_ipv6_is_forbidden() {
        // ::ffff:a.b.c.d connects to the embedded V4 address, so the V4 ruleset
        // must apply — these used to slip past the V6-only checks.
        assert!(host_is_forbidden("::ffff:127.0.0.1"));
        assert!(host_is_forbidden("::ffff:169.254.169.254"));
        assert!(host_is_forbidden("::ffff:10.0.0.1"));
        // A genuine public IPv6 address is still allowed.
        assert!(!host_is_forbidden("2606:4700:4700::1111"));
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
        let r = execute(
            &req,
            CancellationToken::new(),
            ExecuteOptions {
                allow_private_hosts: true,
            },
        )
        .await
        .unwrap();
        assert_eq!(r.body.len(), BODY_CAP_BYTES);
        assert!(r.truncated);
    }
}
