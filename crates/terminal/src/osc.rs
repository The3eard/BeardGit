//! OSC 7 escape sequence parser for terminal cwd auto-detection.
//!
//! Modern shells emit `\x1b]7;file://hostname/path/to/cwd\x07` (or `\x1b\\`
//! as string terminator) when the working directory changes. This module scans
//! raw PTY output for these sequences and extracts the cwd path.

/// Result of scanning a byte buffer for OSC 7 sequences.
#[derive(Debug, Clone, PartialEq)]
pub struct Osc7ScanResult {
    /// Extracted cwd path, if a complete OSC 7 sequence was found.
    /// When multiple sequences are present in a single scan, this is the
    /// last one encountered (reflects the most recent `cd`).
    pub cwd: Option<String>,
    /// Number of bytes from the tail of the combined `pending + chunk`
    /// buffer that form an incomplete OSC 7 prefix and must be carried
    /// over to the next read chunk.
    pub pending_bytes: usize,
}

/// Scan a byte buffer for OSC 7 escape sequences.
///
/// OSC 7 prefix is `ESC ] 7 ;`. We look for it byte-by-byte rather than via
/// a constant so the inner loop stays branch-free on non-ESC bytes.
///
/// `pending` contains leftover bytes from the previous chunk that may form the
/// start of an OSC 7 sequence split across reads. The function returns the
/// extracted cwd (if any) and how many trailing bytes should be carried over.
pub fn scan_osc7(pending: &[u8], chunk: &[u8]) -> Osc7ScanResult {
    // Combine pending bytes from the previous chunk with the new chunk
    let combined: Vec<u8> = if pending.is_empty() {
        chunk.to_vec()
    } else {
        let mut v = Vec::with_capacity(pending.len() + chunk.len());
        v.extend_from_slice(pending);
        v.extend_from_slice(chunk);
        v
    };

    let mut last_cwd: Option<String> = None;
    let mut i = 0;

    while i < combined.len() {
        // Look for ESC ] 7 ; prefix
        if combined[i] == 0x1b && combined.get(i + 1) == Some(&b']') {
            if combined.get(i + 2) == Some(&b'7') && combined.get(i + 3) == Some(&b';') {
                let seq_start = i;
                let url_start = i + 4;

                // Search for terminator: BEL (\x07) or ST (\x1b\\)
                let mut j = url_start;
                let mut found_end = false;
                while j < combined.len() {
                    if combined[j] == 0x07 {
                        // BEL terminator
                        let url_bytes = &combined[url_start..j];
                        if let Ok(url_str) = std::str::from_utf8(url_bytes)
                            && let Some(path) = decode_file_url(url_str)
                        {
                            last_cwd = Some(path);
                        }
                        i = j + 1;
                        found_end = true;
                        break;
                    } else if combined[j] == 0x1b && combined.get(j + 1) == Some(&b'\\') {
                        // ST terminator
                        let url_bytes = &combined[url_start..j];
                        if let Ok(url_str) = std::str::from_utf8(url_bytes)
                            && let Some(path) = decode_file_url(url_str)
                        {
                            last_cwd = Some(path);
                        }
                        i = j + 2;
                        found_end = true;
                        break;
                    }
                    j += 1;
                }

                if !found_end {
                    // Incomplete sequence at end of buffer — carry over
                    let pending_count = combined.len() - seq_start;
                    return Osc7ScanResult {
                        cwd: last_cwd,
                        pending_bytes: pending_count,
                    };
                }

                continue; // i already advanced past the terminator
            } else if combined.get(i + 2).is_none() || combined.get(i + 3).is_none() {
                // We have `ESC ]` at the very end — could be the start of OSC 7.
                // Carry over conservatively to next chunk.
                let pending_count = combined.len() - i;
                return Osc7ScanResult {
                    cwd: last_cwd,
                    pending_bytes: pending_count,
                };
            }
        }

        i += 1;
    }

    Osc7ScanResult {
        cwd: last_cwd,
        pending_bytes: 0,
    }
}

/// Decode a `file://` URL path, handling percent-encoding.
///
/// Strips the `file://hostname` prefix and decodes `%XX` sequences.
/// Returns `None` if the URL is malformed.
fn decode_file_url(url: &str) -> Option<String> {
    // Must start with "file://"
    let rest = url.strip_prefix("file://")?;

    // Find the path start — skip the hostname (everything before the first '/')
    let path_start = rest.find('/')?;
    let encoded_path = &rest[path_start..];

    // Decode percent-encoding
    let mut decoded = String::with_capacity(encoded_path.len());
    let mut bytes = encoded_path.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next()?;
            let lo = bytes.next()?;
            let hex = [hi, lo];
            let s = std::str::from_utf8(&hex).ok()?;
            let byte = u8::from_str_radix(s, 16).ok()?;
            decoded.push(byte as char);
        } else {
            decoded.push(b as char);
        }
    }

    if decoded.is_empty() {
        None
    } else {
        Some(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_complete_osc7_zsh_format() {
        // Standard zsh OSC 7 with BEL terminator
        let data = b"\x1b]7;file://MacBook-Pro.local/Users/adolfo/Projects/BeardGit\x07";
        let result = scan_osc7(&[], data);
        assert_eq!(
            result.cwd.as_deref(),
            Some("/Users/adolfo/Projects/BeardGit")
        );
        assert_eq!(result.pending_bytes, 0);
    }

    #[test]
    fn parse_osc7_with_st_terminator() {
        // OSC 7 with ST (\x1b\\) terminator
        let data = b"\x1b]7;file://localhost/home/user/code\x1b\\";
        let result = scan_osc7(&[], data);
        assert_eq!(result.cwd.as_deref(), Some("/home/user/code"));
        assert_eq!(result.pending_bytes, 0);
    }

    #[test]
    fn parse_osc7_with_percent_encoding() {
        // Path with spaces encoded as %20
        let data = b"\x1b]7;file://host/Users/user/My%20Projects/app\x07";
        let result = scan_osc7(&[], data);
        assert_eq!(result.cwd.as_deref(), Some("/Users/user/My Projects/app"));
    }

    #[test]
    fn parse_osc7_embedded_in_output() {
        // OSC 7 surrounded by regular terminal output
        let data = b"hello world\x1b]7;file://host/tmp/dir\x07more output";
        let result = scan_osc7(&[], data);
        assert_eq!(result.cwd.as_deref(), Some("/tmp/dir"));
        assert_eq!(result.pending_bytes, 0);
    }

    #[test]
    fn parse_osc7_split_across_chunks() {
        // First chunk ends mid-sequence
        let chunk1 = b"output\x1b]7;file://host/tm";
        let result1 = scan_osc7(&[], chunk1);
        assert!(result1.cwd.is_none());
        // pending_bytes should cover from \x1b to end
        assert!(result1.pending_bytes > 0);

        // Second chunk completes the sequence
        let pending = &chunk1[chunk1.len() - result1.pending_bytes..];
        let chunk2 = b"p/split\x07rest";
        let result2 = scan_osc7(pending, chunk2);
        assert_eq!(result2.cwd.as_deref(), Some("/tmp/split"));
        assert_eq!(result2.pending_bytes, 0);
    }

    #[test]
    fn no_osc7_returns_none() {
        let data = b"regular terminal output with no escape sequences\n";
        let result = scan_osc7(&[], data);
        assert!(result.cwd.is_none());
        assert_eq!(result.pending_bytes, 0);
    }

    #[test]
    fn incomplete_osc7_at_end_carries_over() {
        // Chunk ends with an incomplete OSC 7 prefix
        let data = b"output\x1b]7;file://ho";
        let result = scan_osc7(&[], data);
        assert!(result.cwd.is_none());
        // The incomplete sequence should be marked as pending
        assert!(result.pending_bytes > 0);
    }

    #[test]
    fn multiple_osc7_returns_last() {
        // Two OSC 7 sequences in one chunk — last one wins (most recent cd)
        let data = b"\x1b]7;file://h/old/path\x07\x1b]7;file://h/new/path\x07";
        let result = scan_osc7(&[], data);
        assert_eq!(result.cwd.as_deref(), Some("/new/path"));
    }

    #[test]
    fn decode_file_url_strips_host() {
        assert_eq!(
            decode_file_url("file://MacBook-Pro.local/Users/me"),
            Some("/Users/me".to_string())
        );
    }

    #[test]
    fn decode_file_url_handles_localhost() {
        assert_eq!(
            decode_file_url("file://localhost/home/user"),
            Some("/home/user".to_string())
        );
    }

    #[test]
    fn decode_file_url_handles_empty_host() {
        assert_eq!(
            decode_file_url("file:///tmp/dir"),
            Some("/tmp/dir".to_string())
        );
    }

    #[test]
    fn decode_file_url_decodes_percent_encoding() {
        assert_eq!(
            decode_file_url("file://h/path%20with%20spaces/dir%2Fname"),
            Some("/path with spaces/dir/name".to_string())
        );
    }

    #[test]
    fn decode_file_url_rejects_non_file_scheme() {
        assert_eq!(decode_file_url("http://example.com/path"), None);
    }

    #[test]
    fn decode_file_url_rejects_garbage() {
        assert_eq!(decode_file_url("not a url"), None);
    }
}
