//! Generates a JS `fetch(...)` call.

use crate::types::{HttpMethod, ResolvedRequest};

pub fn generate(req: &ResolvedRequest) -> String {
    let mut headers_obj = String::from("{");
    for (i, (k, v)) in req.headers.iter().enumerate() {
        if i > 0 {
            headers_obj.push_str(", ");
        }
        headers_obj.push_str(&format!("{}: {}", js_string(k), js_string(v)));
    }
    headers_obj.push('}');

    let mut opts = vec![format!("method: {}", js_string(req.method.as_str()))];
    if !req.headers.is_empty() {
        opts.push(format!("headers: {headers_obj}"));
    }
    if let Some(b) = &req.body {
        opts.push(format!("body: {}", js_string(b)));
    }
    let opts_str = if opts.len() == 1 && req.method == HttpMethod::Get && req.body.is_none() {
        String::new()
    } else {
        format!(", {{{}}}", opts.join(", "))
    };
    format!("fetch({}{})", js_string(&req.url), opts_str)
}

fn js_string(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_get_no_options() {
        let r = ResolvedRequest {
            method: HttpMethod::Get,
            url: "https://x".into(),
            ..Default::default()
        };
        assert_eq!(generate(&r), "fetch(\"https://x\")");
    }

    #[test]
    fn post_with_body() {
        let r = ResolvedRequest {
            method: HttpMethod::Post,
            url: "https://x".into(),
            headers: vec![("A".into(), "1".into())],
            body: Some("{}".into()),
            ..Default::default()
        };
        let s = generate(&r);
        assert!(s.contains("method: \"POST\""));
        assert!(s.contains("\"A\": \"1\""));
        assert!(s.contains("body: \"{}\""));
    }
}
