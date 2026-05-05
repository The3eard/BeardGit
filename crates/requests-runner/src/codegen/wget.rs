//! Generates a `wget` command.

use crate::types::{HttpMethod, ResolvedRequest};

pub fn generate(req: &ResolvedRequest) -> String {
    let mut parts = vec!["wget".to_string()];
    if req.method != HttpMethod::Get {
        parts.push(format!("--method={}", req.method.as_str()));
    }
    for (k, v) in &req.headers {
        parts.push(format!("--header={}", shell_quote(&format!("{k}: {v}"))));
    }
    if let Some(b) = &req.body {
        parts.push(format!("--body-data={}", shell_quote(b)));
    }
    parts.push(shell_quote(&req.url));
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
    fn post_with_body() {
        let r = ResolvedRequest {
            method: HttpMethod::Post,
            url: "https://x".into(),
            body: Some("{}".into()),
            ..Default::default()
        };
        let s = generate(&r);
        assert!(s.contains("--method=POST"));
        assert!(s.contains("--body-data='{}'"));
        assert!(s.ends_with("'https://x'"));
    }
}
