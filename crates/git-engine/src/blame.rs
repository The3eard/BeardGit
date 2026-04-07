//! Blame and file history operations.
//!
//! Extends [`Repository`] with methods to retrieve per-line blame information
//! and commit history for individual files. Both operations shell out to the
//! system `git` binary (blame uses `--line-porcelain`, history uses `--follow`
//! for rename tracking).

use std::collections::HashMap;

use serde::Serialize;

use crate::error::GitError;
use crate::repository::Repository;

/// A single line of blame output, associating a source line with the commit
/// that last modified it.
#[derive(Debug, Clone, Serialize)]
pub struct BlameLine {
    /// 1-based line number in the final file.
    pub line_num: u32,
    /// The text content of this line.
    pub content: String,
    /// Full commit OID (40 hex chars) that last changed this line.
    pub oid: String,
    /// Author name of the commit.
    pub author: String,
    /// Author email (without angle brackets).
    pub email: String,
    /// Author timestamp as Unix epoch seconds.
    pub timestamp: i64,
    /// First line of the commit message.
    pub summary: String,
}

/// An entry in a file's commit history, including diff stats and rename info.
#[derive(Debug, Clone, Serialize)]
pub struct FileHistoryEntry {
    /// Full commit OID.
    pub oid: String,
    /// First line of the commit message.
    pub message: String,
    /// Author name.
    pub author: String,
    /// ISO-8601 date string (e.g. `2025-04-01 12:00:00 +0000`).
    pub date: String,
    /// Number of lines added in this commit for the file.
    pub additions: usize,
    /// Number of lines removed in this commit for the file.
    pub deletions: usize,
    /// Previous path if the file was renamed in this commit.
    pub old_path: Option<String>,
}

impl Repository {
    /// Get per-line blame information for a file, optionally at a specific commit.
    ///
    /// Shells out to `git blame --line-porcelain` and parses the output into
    /// structured [`BlameLine`] entries. When `oid` is `Some`, blame is computed
    /// at that revision; otherwise HEAD is used.
    pub fn blame_file(&self, path: &str, oid: Option<&str>) -> Result<Vec<BlameLine>, GitError> {
        let mut args = vec!["blame", "--line-porcelain"];
        if let Some(rev) = oid {
            args.push(rev);
        }
        args.push("--");
        args.push(path);

        let result = self.git_cmd(&args)?;
        if !result.success {
            return Err(GitError::RepoNotFound(result.stderr));
        }

        Ok(parse_blame_porcelain(&result.stdout))
    }

    /// Get the commit history for a specific file with rename tracking.
    ///
    /// Uses `git log --follow` to follow file renames. Returns up to `limit`
    /// entries (default 100), each with diff stats (additions/deletions) and
    /// optional rename information.
    pub fn file_history(
        &self,
        path: &str,
        limit: Option<u32>,
    ) -> Result<Vec<FileHistoryEntry>, GitError> {
        let limit_str = limit.unwrap_or(100).to_string();
        let max_count_arg = format!("--max-count={limit_str}");
        let result = self.git_cmd(&[
            "log",
            "--follow",
            &max_count_arg,
            "--format=%H|%s|%an|%ai",
            "--numstat",
            "--",
            path,
        ])?;

        if !result.success {
            return Err(GitError::RepoNotFound(result.stderr));
        }

        Ok(parse_file_history(&result.stdout))
    }
}

/// Parse `git blame --line-porcelain` output into structured blame lines.
///
/// The porcelain format outputs a header block per line, where each block starts
/// with `<sha> <orig-line> <final-line> [<num-lines>]` followed by key-value
/// header lines, and ends with `\t<content>`. For repeated commits only the SHA
/// line and content line appear (no full header), so we cache commit metadata.
fn parse_blame_porcelain(output: &str) -> Vec<BlameLine> {
    let mut lines = Vec::new();
    let mut current_oid = String::new();
    let mut current_author = String::new();
    let mut current_email = String::new();
    let mut current_timestamp: i64 = 0;
    let mut current_summary = String::new();
    let mut line_num: u32 = 0;

    // Cache: oid -> (author, email, timestamp, summary) for repeated commits.
    let mut commit_cache: HashMap<String, (String, String, i64, String)> = HashMap::new();

    let mut in_header = false;

    for raw_line in output.lines() {
        if let Some(content) = raw_line.strip_prefix('\t') {
            // Content line — end of this block.
            // strip leading tab

            // Look up from cache if we didn't see a full header for this block.
            if current_author.is_empty()
                && let Some(cached) = commit_cache.get(&current_oid) {
                    current_author.clone_from(&cached.0);
                    current_email.clone_from(&cached.1);
                    current_timestamp = cached.2;
                    current_summary.clone_from(&cached.3);
                }

            lines.push(BlameLine {
                line_num,
                content: content.to_string(),
                oid: current_oid.clone(),
                author: current_author.clone(),
                email: current_email.clone(),
                timestamp: current_timestamp,
                summary: current_summary.clone(),
            });

            // Cache this commit's info on first full appearance.
            commit_cache.entry(current_oid.clone()).or_insert_with(|| {
                (
                    current_author.clone(),
                    current_email.clone(),
                    current_timestamp,
                    current_summary.clone(),
                )
            });

            // Reset for next block.
            in_header = false;
            current_author.clear();
            current_email.clear();
            current_timestamp = 0;
            current_summary.clear();
        } else if !in_header
            && raw_line.len() >= 40
            && raw_line
                .as_bytes()
                .iter()
                .take(40)
                .all(|b| b.is_ascii_hexdigit())
        {
            // SHA line — start of new block.
            // Format: <sha> <orig-line> <final-line> [<num-lines>]
            let parts: Vec<&str> = raw_line.split_whitespace().collect();
            current_oid = parts[0].to_string();
            line_num = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
            in_header = true;
        } else if in_header {
            // Header key-value lines.
            if let Some(val) = raw_line.strip_prefix("author ") {
                current_author = val.to_string();
            } else if let Some(val) = raw_line.strip_prefix("author-mail ") {
                current_email = val.trim_matches(|c| c == '<' || c == '>').to_string();
            } else if let Some(val) = raw_line.strip_prefix("author-time ") {
                current_timestamp = val.parse().unwrap_or(0);
            } else if let Some(val) = raw_line.strip_prefix("summary ") {
                current_summary = val.to_string();
            }
            // Ignore other header lines (committer-*, boundary, previous, filename).
        }
    }

    lines
}

/// Parse `git log --follow --format=%H|%s|%an|%ai --numstat` output into
/// structured file history entries.
///
/// Each commit appears as a `%H|%s|%an|%ai` line followed by a blank line and
/// a numstat line (`additions\tdeletions\tpath`). Renames appear as
/// `old => new` in the path column.
fn parse_file_history(output: &str) -> Vec<FileHistoryEntry> {
    let mut entries = Vec::new();
    let mut current: Option<FileHistoryEntry> = None;

    for line in output.lines() {
        if line.is_empty() {
            continue;
        }

        // Try to parse as a commit line (has | separators and starts with hex).
        let parts: Vec<&str> = line.splitn(4, '|').collect();
        if parts.len() == 4
            && parts[0].len() >= 7
            && parts[0].chars().all(|c| c.is_ascii_hexdigit())
        {
            // Save previous entry.
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(FileHistoryEntry {
                oid: parts[0].to_string(),
                message: parts[1].to_string(),
                author: parts[2].to_string(),
                date: parts[3].to_string(),
                additions: 0,
                deletions: 0,
                old_path: None,
            });
        } else if let Some(ref mut entry) = current {
            // Try to parse as numstat line: "5\t3\tpath" or "5\t3\told => new".
            let stat_parts: Vec<&str> = line.split('\t').collect();
            if stat_parts.len() >= 3 {
                entry.additions = stat_parts[0].parse().unwrap_or(0);
                entry.deletions = stat_parts[1].parse().unwrap_or(0);
                // Check for rename syntax.
                if let Some(rename_pos) = stat_parts[2].find(" => ") {
                    let old = &stat_parts[2][..rename_pos];
                    entry.old_path = Some(old.to_string());
                }
            }
        }
    }

    // Don't forget the last entry.
    if let Some(entry) = current {
        entries.push(entry);
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_blame_porcelain() {
        let output = "\
abc1234567890123456789012345678901234567 1 1 2
author Alice
author-mail <alice@example.com>
author-time 1700000000
author-tz +0000
committer Alice
committer-mail <alice@example.com>
committer-time 1700000000
committer-tz +0000
summary feat: initial commit
filename src/main.rs
\tfn main() {
abc1234567890123456789012345678901234567 2 2
\t    println!(\"hello\");
def1234567890123456789012345678901234567 3 3 1
author Bob
author-mail <bob@example.com>
author-time 1700100000
author-tz +0000
committer Bob
committer-mail <bob@example.com>
committer-time 1700100000
committer-tz +0000
summary fix: add closing brace
filename src/main.rs
\t}";

        let lines = parse_blame_porcelain(output);
        assert_eq!(lines.len(), 3);

        // First line — full header from commit abc...
        assert_eq!(lines[0].line_num, 1);
        assert_eq!(lines[0].content, "fn main() {");
        assert_eq!(lines[0].oid, "abc1234567890123456789012345678901234567");
        assert_eq!(lines[0].author, "Alice");
        assert_eq!(lines[0].email, "alice@example.com");
        assert_eq!(lines[0].timestamp, 1_700_000_000);
        assert_eq!(lines[0].summary, "feat: initial commit");

        // Second line — same commit, cached (short header).
        assert_eq!(lines[1].line_num, 2);
        assert_eq!(lines[1].content, "    println!(\"hello\");");
        assert_eq!(lines[1].oid, lines[0].oid);
        assert_eq!(lines[1].author, "Alice");
        assert_eq!(lines[1].timestamp, 1_700_000_000);

        // Third line — different commit.
        assert_eq!(lines[2].line_num, 3);
        assert_eq!(lines[2].content, "}");
        assert_eq!(lines[2].oid, "def1234567890123456789012345678901234567");
        assert_eq!(lines[2].author, "Bob");
        assert_eq!(lines[2].timestamp, 1_700_100_000);
        assert_eq!(lines[2].summary, "fix: add closing brace");
    }

    #[test]
    fn test_parse_blame_single_commit() {
        let output = "\
aaa1234567890123456789012345678901234567 1 1 3
author Carol
author-mail <carol@example.com>
author-time 1700200000
author-tz +0000
committer Carol
committer-mail <carol@example.com>
committer-time 1700200000
committer-tz +0000
summary chore: single commit file
filename file.txt
\tline one
aaa1234567890123456789012345678901234567 2 2
\tline two
aaa1234567890123456789012345678901234567 3 3
\tline three";

        let lines = parse_blame_porcelain(output);
        assert_eq!(lines.len(), 3);

        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line.oid, "aaa1234567890123456789012345678901234567");
            assert_eq!(line.author, "Carol");
            assert_eq!(line.timestamp, 1_700_200_000);
            assert_eq!(line.line_num, (i + 1) as u32);
        }

        assert_eq!(lines[0].content, "line one");
        assert_eq!(lines[1].content, "line two");
        assert_eq!(lines[2].content, "line three");
    }

    #[test]
    fn test_parse_blame_empty() {
        let lines = parse_blame_porcelain("");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_parse_file_history() {
        let output = "\
abc1234567890123456789012345678901234567|feat: add feature|John|2025-04-01 12:00:00 +0000

5\t3\tsrc/file.rs

def1234567890123456789012345678901234567|fix: bug fix|Jane|2025-04-02 10:00:00 +0000

2\t1\tsrc/file.rs
";

        let entries = parse_file_history(output);
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].oid, "abc1234567890123456789012345678901234567");
        assert_eq!(entries[0].message, "feat: add feature");
        assert_eq!(entries[0].author, "John");
        assert_eq!(entries[0].date, "2025-04-01 12:00:00 +0000");
        assert_eq!(entries[0].additions, 5);
        assert_eq!(entries[0].deletions, 3);
        assert!(entries[0].old_path.is_none());

        assert_eq!(entries[1].oid, "def1234567890123456789012345678901234567");
        assert_eq!(entries[1].message, "fix: bug fix");
        assert_eq!(entries[1].author, "Jane");
        assert_eq!(entries[1].additions, 2);
        assert_eq!(entries[1].deletions, 1);
    }

    #[test]
    fn test_parse_file_history_with_rename() {
        let output = "\
abc1234567890123456789012345678901234567|refactor: rename module|Alice|2025-04-03 08:00:00 +0000

10\t5\tsrc/old_file.rs => src/new_file.rs
";

        let entries = parse_file_history(output);
        assert_eq!(entries.len(), 1);

        assert_eq!(entries[0].additions, 10);
        assert_eq!(entries[0].deletions, 5);
        assert_eq!(entries[0].old_path.as_deref(), Some("src/old_file.rs"));
    }

    #[test]
    fn test_parse_file_history_empty() {
        let entries = parse_file_history("");
        assert!(entries.is_empty());
    }
}
