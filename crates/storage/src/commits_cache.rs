//! Commit cache: stores and retrieves serialized git commits in SQLite.

use serde::{Deserialize, Serialize};

use crate::{database::Database, error::StorageError};

/// A flattened git commit record stored in the `commits_cache` SQLite table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCommit {
    /// Full 40-character hex SHA of the commit.
    pub oid: String,
    /// First line of the commit message.
    pub summary: String,
    /// Remaining commit message body (may be empty).
    pub body: String,
    /// Author display name.
    pub author: String,
    /// Author email address.
    pub email: String,
    /// Unix timestamp (seconds since epoch) of the author date.
    pub timestamp: i64,
    /// Hex SHAs of parent commits (empty for root commits).
    pub parents: Vec<String>,
    /// Branch/tag ref names pointing at this commit (e.g. `["refs/heads/main"]`).
    pub refs: Vec<String>,
}

impl Database {
    /// Batch-insert commits for a repository using a single transaction.
    pub fn insert_commits(
        &self,
        repo_path: &str,
        commits: &[CachedCommit],
    ) -> Result<(), StorageError> {
        let conn = &self.conn;
        conn.execute_batch("BEGIN")?;
        {
            let mut stmt = conn.prepare(
                "INSERT OR REPLACE INTO commits_cache
                    (repo_path, oid, summary, body, author, email, timestamp, parents, refs)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;

            for commit in commits {
                let parents_json = serde_json::to_string(&commit.parents)?;
                let refs_json = serde_json::to_string(&commit.refs)?;
                stmt.execute(rusqlite::params![
                    repo_path,
                    commit.oid,
                    commit.summary,
                    commit.body,
                    commit.author,
                    commit.email,
                    commit.timestamp,
                    parents_json,
                    refs_json,
                ])?;
            }
        }
        conn.execute_batch("COMMIT")?;
        Ok(())
    }

    /// Retrieve commits for a repo, ordered by timestamp DESC, with pagination.
    pub fn get_commits(
        &self,
        repo_path: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<CachedCommit>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT oid, summary, body, author, email, timestamp, parents, refs
             FROM commits_cache
             WHERE repo_path = ?1
             ORDER BY timestamp DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let rows = stmt.query_map(rusqlite::params![repo_path, limit, offset], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;

        let mut commits = Vec::new();
        for row in rows {
            let (oid, summary, body, author, email, timestamp, parents_json, refs_json) = row?;
            let parents: Vec<String> = serde_json::from_str(&parents_json)?;
            let refs: Vec<String> = serde_json::from_str(&refs_json)?;
            commits.push(CachedCommit {
                oid,
                summary,
                body,
                author,
                email,
                timestamp,
                parents,
                refs,
            });
        }

        Ok(commits)
    }

    /// Return the total number of cached commits for a repository.
    pub fn get_commit_count(&self, repo_path: &str) -> Result<i64, StorageError> {
        let count = self.conn.query_row(
            "SELECT COUNT(*) FROM commits_cache WHERE repo_path = ?1",
            rusqlite::params![repo_path],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Delete all cached commits for a repository.
    pub fn clear_commits(&self, repo_path: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "DELETE FROM commits_cache WHERE repo_path = ?1",
            rusqlite::params![repo_path],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    fn make_commit(oid: &str, summary: &str, timestamp: i64) -> CachedCommit {
        CachedCommit {
            oid: oid.to_string(),
            summary: summary.to_string(),
            body: String::new(),
            author: "Test Author".to_string(),
            email: "author@example.com".to_string(),
            timestamp,
            parents: vec![],
            refs: vec![],
        }
    }

    #[test]
    fn test_insert_and_retrieve_commits() {
        let db = Database::open_in_memory().unwrap();
        let repo = "/repos/test";

        let commits = vec![
            make_commit("aaa", "first commit", 1000),
            make_commit("bbb", "second commit", 2000),
        ];
        db.insert_commits(repo, &commits).unwrap();

        let retrieved = db.get_commits(repo, 0, 10).unwrap();
        assert_eq!(retrieved.len(), 2);
        // ordered by timestamp DESC — newest first
        assert_eq!(retrieved[0].oid, "bbb");
        assert_eq!(retrieved[1].oid, "aaa");
    }

    #[test]
    fn test_get_commit_count() {
        let db = Database::open_in_memory().unwrap();

        db.insert_commits(
            "/repos/a",
            &[
                make_commit("c1", "commit 1", 100),
                make_commit("c2", "commit 2", 200),
            ],
        )
        .unwrap();
        db.insert_commits("/repos/b", &[make_commit("c3", "commit 3", 300)])
            .unwrap();

        assert_eq!(db.get_commit_count("/repos/a").unwrap(), 2);
        assert_eq!(db.get_commit_count("/repos/b").unwrap(), 1);
        assert_eq!(db.get_commit_count("/repos/c").unwrap(), 0);
    }

    #[test]
    fn test_pagination() {
        let db = Database::open_in_memory().unwrap();
        let repo = "/repos/paginated";

        let commits: Vec<CachedCommit> = (0..20)
            .map(|i| make_commit(&format!("oid{:02}", i), &format!("commit {}", i), i as i64))
            .collect();
        db.insert_commits(repo, &commits).unwrap();

        // First page: 5 items starting at offset 0
        let page1 = db.get_commits(repo, 0, 5).unwrap();
        assert_eq!(page1.len(), 5);

        // Second page: 5 items starting at offset 5
        let page2 = db.get_commits(repo, 5, 5).unwrap();
        assert_eq!(page2.len(), 5);

        // Pages should not overlap
        let ids1: Vec<&str> = page1.iter().map(|c| c.oid.as_str()).collect();
        let ids2: Vec<&str> = page2.iter().map(|c| c.oid.as_str()).collect();
        for id in &ids2 {
            assert!(!ids1.contains(id), "pages should not overlap");
        }

        // Last page: offset 15, only 5 items remain
        let last_page = db.get_commits(repo, 15, 10).unwrap();
        assert_eq!(last_page.len(), 5);
    }

    #[test]
    fn test_clear_commits() {
        let db = Database::open_in_memory().unwrap();
        let repo = "/repos/clear";

        db.insert_commits(
            repo,
            &[
                make_commit("x1", "commit x1", 100),
                make_commit("x2", "commit x2", 200),
            ],
        )
        .unwrap();
        assert_eq!(db.get_commit_count(repo).unwrap(), 2);

        db.clear_commits(repo).unwrap();
        assert_eq!(db.get_commit_count(repo).unwrap(), 0);
    }
}
