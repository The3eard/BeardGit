//! Variable resolution. Substitutes `{{var}}` placeholders in URL, headers,
//! and body using a precedence order.
//!
//! Precedence (highest first):
//!   1. `ctx.overrides`              — per-run UI override
//!   2. `ctx.env_secrets`            — values resolved from credential store
//!   3. `ctx.env_vars`               — values from `_env/<active>.json`
//!
//! Anything that is referenced and not present raises
//! [`RequestsError::UnresolvedVar`].

use std::collections::BTreeSet;

use crate::{
    error::RequestsError,
    types::{ParsedRequest, ResolveCtx, ResolvedRequest},
};

const MAX_DEPTH: usize = 32;

pub fn resolve(req: &ParsedRequest, ctx: &ResolveCtx) -> Result<ResolvedRequest, RequestsError> {
    let url = expand(&req.url, ctx, &mut BTreeSet::new(), 0)?;
    let mut headers = Vec::with_capacity(req.headers.len());
    for (k, v) in &req.headers {
        let kk = expand(k, ctx, &mut BTreeSet::new(), 0)?;
        let vv = expand(v, ctx, &mut BTreeSet::new(), 0)?;
        headers.push((kk, vv));
    }
    let body = match &req.body {
        Some(b) => Some(expand(b, ctx, &mut BTreeSet::new(), 0)?),
        None => None,
    };
    Ok(ResolvedRequest {
        name: req.name.clone(),
        method: req.method.clone().unwrap_or_default(),
        url,
        headers,
        body,
    })
}

fn expand(
    s: &str,
    ctx: &ResolveCtx,
    seen: &mut BTreeSet<String>,
    depth: usize,
) -> Result<String, RequestsError> {
    if depth > MAX_DEPTH {
        return Err(RequestsError::CycleDetected {
            vars: seen.iter().cloned().collect(),
        });
    }
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len()
            && bytes[i] == b'{'
            && bytes[i + 1] == b'{'
            && let Some(end) = find_close(s, i + 2)
        {
            let name = s[i + 2..end].trim();
            if name.is_empty() {
                out.push_str(&s[i..i + 2]);
                i += 2;
                continue;
            }
            if seen.contains(name) {
                let mut vars: Vec<String> = seen.iter().cloned().collect();
                vars.push(name.to_string());
                return Err(RequestsError::CycleDetected { vars });
            }
            let val = lookup(name, ctx)?;
            seen.insert(name.to_string());
            let resolved = expand(&val, ctx, seen, depth + 1)?;
            seen.remove(name);
            out.push_str(&resolved);
            i = end + 2;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    Ok(out)
}

fn find_close(s: &str, from: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = from;
    while i + 1 < bytes.len() {
        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn lookup(name: &str, ctx: &ResolveCtx) -> Result<String, RequestsError> {
    if let Some(v) = ctx.overrides.get(name) {
        return Ok(v.clone());
    }
    if let Some(v) = ctx.env_secrets.get(name) {
        return Ok(v.clone());
    }
    if let Some(v) = ctx.env_vars.get(name) {
        return Ok(v.clone());
    }
    Err(RequestsError::UnresolvedVar {
        name: name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HttpMethod;

    fn req(url: &str) -> ParsedRequest {
        ParsedRequest {
            url: url.into(),
            method: Some(HttpMethod::Get),
            ..Default::default()
        }
    }

    fn ctx_with_vars(pairs: &[(&str, &str)]) -> ResolveCtx {
        let mut c = ResolveCtx::default();
        for (k, v) in pairs {
            c.env_vars.insert((*k).into(), (*v).into());
        }
        c
    }

    #[test]
    fn substitutes_single_var() {
        let r = resolve(
            &req("https://{{host}}/x"),
            &ctx_with_vars(&[("host", "api.example.com")]),
        )
        .unwrap();
        assert_eq!(r.url, "https://api.example.com/x");
    }

    #[test]
    fn multiple_vars() {
        let r = resolve(
            &req("{{a}}/{{b}}"),
            &ctx_with_vars(&[("a", "x"), ("b", "y")]),
        )
        .unwrap();
        assert_eq!(r.url, "x/y");
    }

    #[test]
    fn unresolved_errors() {
        let err = resolve(&req("{{ghost}}"), &ResolveCtx::default()).unwrap_err();
        assert!(matches!(err, RequestsError::UnresolvedVar { .. }));
    }

    #[test]
    fn override_beats_secret_beats_var() {
        let mut ctx = ctx_with_vars(&[("k", "from_var")]);
        ctx.env_secrets.insert("k".into(), "from_secret".into());
        let r = resolve(&req("{{k}}"), &ctx).unwrap();
        assert_eq!(r.url, "from_secret");

        ctx.overrides.insert("k".into(), "from_override".into());
        let r = resolve(&req("{{k}}"), &ctx).unwrap();
        assert_eq!(r.url, "from_override");
    }

    #[test]
    fn nested_var_resolves() {
        let mut ctx = ResolveCtx::default();
        ctx.env_vars.insert("a".into(), "{{b}}".into());
        ctx.env_vars.insert("b".into(), "final".into());
        let r = resolve(&req("{{a}}"), &ctx).unwrap();
        assert_eq!(r.url, "final");
    }

    #[test]
    fn cycle_detected() {
        let mut ctx = ResolveCtx::default();
        ctx.env_vars.insert("a".into(), "{{b}}".into());
        ctx.env_vars.insert("b".into(), "{{a}}".into());
        let err = resolve(&req("{{a}}"), &ctx).unwrap_err();
        assert!(matches!(err, RequestsError::CycleDetected { .. }));
    }

    #[test]
    fn substitutes_in_headers_and_body() {
        let mut ctx = ResolveCtx::default();
        ctx.env_vars.insert("v".into(), "X".into());
        let pr = ParsedRequest {
            method: Some(HttpMethod::Post),
            url: "https://h/{{v}}".into(),
            headers: vec![("H".into(), "{{v}}".into())],
            body: Some("body {{v}}".into()),
            ..Default::default()
        };
        let r = resolve(&pr, &ctx).unwrap();
        assert_eq!(r.url, "https://h/X");
        assert_eq!(r.headers[0].1, "X");
        assert_eq!(r.body.as_deref(), Some("body X"));
    }
}
