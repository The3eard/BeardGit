//! Parser for `.http` files (REST Client / IntelliJ HTTP Client format).
//!
//! A file is a sequence of one or more **blocks**, each beginning with either:
//!   - the start of file, or
//!   - a separator line `### [optional name]`.
//!
//! Inside a block:
//!   - lines starting with `#` or `//` are comments;
//!   - the first non-comment, non-blank line is the **request line** (`METHOD URL`);
//!   - subsequent lines until a blank line are **headers** (`Name: Value`);
//!   - everything after the first blank line (until the next block) is the **body**.

use crate::{
    error::RequestsError,
    types::{HttpMethod, ParsedRequest},
};

pub fn parse_http_file(src: &str) -> Result<Vec<ParsedRequest>, RequestsError> {
    let mut blocks = vec![];
    let mut current_lines: Vec<(usize, &str)> = vec![]; // (1-based line number, content)
    let mut current_name: Option<String> = None;

    for (idx, line) in src.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(rest) = line.strip_prefix("###") {
            if !current_lines.is_empty() || current_name.is_some() {
                blocks.push((current_name.take(), std::mem::take(&mut current_lines)));
            }
            let name = rest.trim();
            current_name = if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            };
            continue;
        }
        current_lines.push((line_no, line));
    }
    if !current_lines.is_empty() || current_name.is_some() {
        blocks.push((current_name, current_lines));
    }

    let mut out = vec![];
    for (name, lines) in blocks {
        if let Some(req) = parse_block(name, &lines)? {
            out.push(req);
        }
    }
    Ok(out)
}

fn parse_block(
    name: Option<String>,
    lines: &[(usize, &str)],
) -> Result<Option<ParsedRequest>, RequestsError> {
    let mut iter = lines.iter().peekable();

    let mut name_from_comment: Option<String> = None;
    while let Some(&(_, l)) = iter.peek() {
        let trim = l.trim();
        if trim.is_empty() || trim.starts_with('#') || trim.starts_with("//") {
            if let Some(rest) = trim.strip_prefix("# @name") {
                name_from_comment = Some(rest.trim().to_string());
            }
            iter.next();
            continue;
        }
        break;
    }

    let Some(&(req_line_no, req_line)) = iter.next() else {
        return Ok(None);
    };
    let mut parts = req_line.trim().splitn(2, char::is_whitespace);
    let method_str = parts.next().unwrap_or("");
    let url = parts.next().unwrap_or("").trim().to_string();
    let method = HttpMethod::parse(method_str).ok_or(RequestsError::Parse {
        line: req_line_no,
        col: 1,
        reason: format!("unknown HTTP method `{method_str}`"),
    })?;
    if url.is_empty() {
        return Err(RequestsError::Parse {
            line: req_line_no,
            col: method_str.len() + 2,
            reason: "missing URL after method".into(),
        });
    }

    let mut headers = vec![];
    while let Some(&&(line_no, l)) = iter.peek() {
        if l.trim().is_empty() {
            iter.next();
            break;
        }
        let trim = l.trim();
        if trim.starts_with('#') || trim.starts_with("//") {
            iter.next();
            continue;
        }
        let (k, v) = trim.split_once(':').ok_or_else(|| RequestsError::Parse {
            line: line_no,
            col: 1,
            reason: format!("header missing `:` separator: `{trim}`"),
        })?;
        headers.push((k.trim().to_string(), v.trim().to_string()));
        iter.next();
    }

    let body_lines: Vec<&str> = iter.map(|(_, l)| *l).collect();
    let body = if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n").trim_end().to_string())
    };
    let body = body.filter(|s| !s.is_empty());

    Ok(Some(ParsedRequest {
        name: name_from_comment.or(name),
        method: Some(method),
        url,
        headers,
        body,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_get() {
        let src = "GET https://example.com/users\n";
        let r = parse_http_file(src).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].method, Some(HttpMethod::Get));
        assert_eq!(r[0].url, "https://example.com/users");
        assert!(r[0].headers.is_empty());
        assert!(r[0].body.is_none());
    }

    #[test]
    fn parse_post_with_headers_and_body() {
        let src = "POST https://example.com/api\n\
                   Content-Type: application/json\n\
                   Authorization: Bearer abc\n\
                   \n\
                   {\"name\":\"Adolfo\"}\n";
        let r = parse_http_file(src).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].method, Some(HttpMethod::Post));
        assert_eq!(r[0].headers.len(), 2);
        assert_eq!(
            r[0].headers[0],
            ("Content-Type".into(), "application/json".into())
        );
        assert_eq!(r[0].body.as_deref(), Some("{\"name\":\"Adolfo\"}"));
    }

    #[test]
    fn name_from_at_comment() {
        let src = "# @name Create user\nPOST https://x/y\n";
        let r = parse_http_file(src).unwrap();
        assert_eq!(r[0].name.as_deref(), Some("Create user"));
    }

    #[test]
    fn parse_multi_block_with_separator() {
        let src = "### Get\n\
                   GET https://x/a\n\
                   \n\
                   ### Post\n\
                   POST https://x/b\n\
                   \n\
                   {}\n";
        let r = parse_http_file(src).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].name.as_deref(), Some("Get"));
        assert_eq!(r[0].method, Some(HttpMethod::Get));
        assert_eq!(r[1].name.as_deref(), Some("Post"));
        assert_eq!(r[1].body.as_deref(), Some("{}"));
    }

    #[test]
    fn ignore_double_slash_and_hash_comments() {
        let src = "// leading comment\n\
                   # another\n\
                   GET https://x/y\n\
                   // header comment\n\
                   X-A: 1\n";
        let r = parse_http_file(src).unwrap();
        assert_eq!(r[0].headers.len(), 1);
    }

    #[test]
    fn unknown_method_errors_with_line_and_col() {
        let src = "FOO https://x\n";
        let err = parse_http_file(src).unwrap_err();
        match err {
            RequestsError::Parse { line, col, reason } => {
                assert_eq!(line, 1);
                assert_eq!(col, 1);
                assert!(reason.contains("FOO"), "reason: {reason}");
            }
            _ => panic!("expected Parse"),
        }
    }

    #[test]
    fn missing_url_errors() {
        let src = "GET\n";
        let err = parse_http_file(src).unwrap_err();
        assert!(matches!(err, RequestsError::Parse { .. }));
    }

    #[test]
    fn malformed_header_errors() {
        let src = "GET https://x\nNoColon\n";
        let err = parse_http_file(src).unwrap_err();
        assert!(matches!(err, RequestsError::Parse { .. }));
    }

    #[test]
    fn crlf_is_handled() {
        let src = "GET https://x/y\r\nA: 1\r\n";
        let r = parse_http_file(src).unwrap();
        assert_eq!(r[0].headers[0].1, "1");
    }
}

/// Parse a `curl ...` command line into a [`ParsedRequest`]. Best-effort:
/// supports `-X METHOD`, `-H 'name: value'`, `--data-raw 'body'`, and the
/// URL as the first non-flag positional. Designed for the "Paste cURL"
/// affordance, not for general shell-command parsing.
pub fn import_curl(curl_cmd: &str) -> Result<ParsedRequest, RequestsError> {
    let tokens = shell_tokens(curl_cmd).map_err(|e| RequestsError::Parse {
        line: 1,
        col: 1,
        reason: e,
    })?;
    if tokens.first().map(|t| t.as_str()) != Some("curl") {
        return Err(RequestsError::Parse {
            line: 1,
            col: 1,
            reason: "expected `curl` as the first token".into(),
        });
    }
    let mut method: Option<HttpMethod> = None;
    let mut headers: Vec<(String, String)> = vec![];
    let mut body: Option<String> = None;
    let mut url: Option<String> = None;

    let mut i = 1;
    while i < tokens.len() {
        let t = &tokens[i];
        match t.as_str() {
            "-X" | "--request" => {
                let v = tokens.get(i + 1).cloned().unwrap_or_default();
                // An explicit -X with an unrecognized method is an error, not a
                // silent fall-through to GET/POST (which would drop the user's
                // intended method, e.g. a typo'd `POSt` or an unmodeled `PURGE`).
                let Some(m) = HttpMethod::parse(&v) else {
                    return Err(RequestsError::Parse {
                        line: 1,
                        col: 1,
                        reason: format!("unrecognized HTTP method in curl -X: {v}"),
                    });
                };
                method = Some(m);
                i += 2;
            }
            "-H" | "--header" => {
                let v = tokens.get(i + 1).cloned().unwrap_or_default();
                if let Some((k, val)) = v.split_once(':') {
                    headers.push((k.trim().to_string(), val.trim().to_string()));
                }
                i += 2;
            }
            "--data-raw" | "--data" | "-d" | "--data-binary" => {
                body = tokens.get(i + 1).cloned();
                i += 2;
            }
            "--url" => {
                url = tokens.get(i + 1).cloned();
                i += 2;
            }
            other if !other.starts_with('-') => {
                if url.is_none() {
                    url = Some(other.to_string());
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    let url = url.ok_or(RequestsError::Parse {
        line: 1,
        col: 1,
        reason: "no URL found in curl command".into(),
    })?;
    let method = method.or(if body.is_some() {
        Some(HttpMethod::Post)
    } else {
        Some(HttpMethod::Get)
    });
    Ok(ParsedRequest {
        name: None,
        method,
        url,
        headers,
        body,
    })
}

fn shell_tokens(s: &str) -> Result<Vec<String>, String> {
    let mut tokens: Vec<String> = vec![];
    let mut cur = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut iter = s.chars().peekable();
    while let Some(c) = iter.next() {
        match c {
            '\\' if !in_single => {
                if let Some(&nc) = iter.peek() {
                    cur.push(nc);
                    iter.next();
                }
            }
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ' ' | '\t' | '\n' if !in_single && !in_double => {
                if !cur.is_empty() {
                    tokens.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if in_single || in_double {
        return Err("unterminated quote".into());
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    Ok(tokens)
}

#[cfg(test)]
mod import_curl_tests {
    use super::*;

    #[test]
    fn imports_simple_get() {
        let r = import_curl("curl 'https://x/y'").unwrap();
        assert_eq!(r.url, "https://x/y");
        assert_eq!(r.method, Some(HttpMethod::Get));
    }

    #[test]
    fn imports_post_with_header_and_body() {
        let cmd = "curl -X POST 'https://x' -H 'Content-Type: application/json' --data-raw '{}'";
        let r = import_curl(cmd).unwrap();
        assert_eq!(r.method, Some(HttpMethod::Post));
        assert_eq!(
            r.headers,
            vec![("Content-Type".into(), "application/json".into())]
        );
        assert_eq!(r.body.as_deref(), Some("{}"));
    }

    #[test]
    fn missing_url_errors() {
        let err = import_curl("curl -X POST").unwrap_err();
        assert!(matches!(err, RequestsError::Parse { .. }));
    }

    #[test]
    fn unknown_explicit_method_errors() {
        // An explicit -X with an unrecognized method must error rather than
        // silently downgrading to GET/POST.
        let err = import_curl("curl -X PURGE https://example.com/x").unwrap_err();
        assert!(matches!(err, RequestsError::Parse { .. }));
    }
}
