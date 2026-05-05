//! Generates a `curl` command equivalent to the resolved request.

use crate::types::{HttpMethod, ResolvedRequest};

pub fn generate(req: &ResolvedRequest) -> String {
    let mut parts = vec!["curl".to_string()];
    if req.method != HttpMethod::Get {
        parts.push("-X".into());
        parts.push(req.method.as_str().into());
    }
    parts.push(shell_quote(&req.url));
    for (k, v) in &req.headers {
        parts.push("-H".into());
        parts.push(shell_quote(&format!("{k}: {v}")));
    }
    if let Some(body) = &req.body {
        parts.push("--data-raw".into());
        parts.push(shell_quote(body));
    }
    parts.join(" ")
}

fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_get() {
        let r = ResolvedRequest {
            method: HttpMethod::Get,
            url: "https://x/y".into(),
            ..Default::default()
        };
        assert_eq!(generate(&r), "curl 'https://x/y'");
    }

    #[test]
    fn post_with_headers_and_body() {
        let r = ResolvedRequest {
            method: HttpMethod::Post,
            url: "https://x/y".into(),
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: Some("{\"a\":1}".into()),
            ..Default::default()
        };
        let s = generate(&r);
        assert!(s.contains("-X POST"));
        assert!(s.contains("-H 'Content-Type: application/json'"));
        assert!(s.contains("--data-raw '{\"a\":1}'"));
    }

    #[test]
    fn quoted_single_quote_in_url() {
        let r = ResolvedRequest {
            method: HttpMethod::Get,
            url: "https://x/it's".into(),
            ..Default::default()
        };
        assert_eq!(generate(&r), "curl 'https://x/it'\\''s'");
    }
}
