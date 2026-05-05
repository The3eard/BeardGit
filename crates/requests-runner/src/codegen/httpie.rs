//! Generates an `http` (HTTPie) command.

use crate::types::ResolvedRequest;

pub fn generate(req: &ResolvedRequest) -> String {
    let mut parts = vec!["http".to_string()];
    parts.push(req.method.as_str().to_string());
    parts.push(shell_quote(&req.url));
    for (k, v) in &req.headers {
        parts.push(shell_quote(&format!("{k}:{v}")));
    }
    if let Some(b) = &req.body {
        parts.push(format!("<<<{}", shell_quote(b)));
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
    use crate::types::HttpMethod;

    #[test]
    fn get_with_header() {
        let r = ResolvedRequest {
            method: HttpMethod::Get,
            url: "https://x".into(),
            headers: vec![("A".into(), "1".into())],
            ..Default::default()
        };
        let s = generate(&r);
        assert!(s.starts_with("http GET 'https://x'"));
        assert!(s.contains("'A:1'"));
    }
}
