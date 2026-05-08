//! SQLite database wrapper with schema migrations.

use std::path::Path;

use rusqlite::Connection;

use crate::error::StorageError;

/// A thin wrapper around a [`rusqlite::Connection`] that applies schema migrations on open.
pub struct Database {
    pub(crate) conn: Connection,
}

/// Apply performance and concurrency pragmas. WAL mode is silently ignored on
/// in-memory databases, the rest apply universally.
fn configure_connection(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA temp_store = MEMORY;
         PRAGMA foreign_keys = ON;",
    )?;
    Ok(())
}

impl Database {
    /// Open (or create) a SQLite database at the given file path and run migrations.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        configure_connection(&conn)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory SQLite database and run migrations.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        configure_connection(&conn)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<(), StorageError> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;

        if version < 1 {
            self.migrate_v1()?;
        }
        if version < 2 {
            self.migrate_v2()?;
        }

        Ok(())
    }

    fn migrate_v1(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS commits_cache (
                repo_path TEXT NOT NULL,
                oid       TEXT NOT NULL,
                summary   TEXT NOT NULL,
                body      TEXT NOT NULL DEFAULT '',
                author    TEXT NOT NULL,
                email     TEXT NOT NULL DEFAULT '',
                timestamp INTEGER NOT NULL,
                parents   TEXT NOT NULL DEFAULT '[]',
                refs      TEXT NOT NULL DEFAULT '[]',
                PRIMARY KEY (repo_path, oid)
            );

            CREATE INDEX IF NOT EXISTS idx_commits_repo_time
                ON commits_cache(repo_path, timestamp DESC);

            PRAGMA user_version = 1;
            ",
        )?;
        Ok(())
    }

    /// v2 — `parents` / `refs` columns switched from JSON-encoded arrays
    /// (`["a","b"]`) to Unit-Separator (`\x1f`) joined strings. The cache is
    /// purged on upgrade because reconstructing it from libgit2 on next
    /// launch is faster than parsing both encodings indefinitely.
    fn migrate_v2(&self) -> Result<(), StorageError> {
        self.conn.execute_batch(
            "
            DELETE FROM commits_cache;
            PRAGMA user_version = 2;
            ",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory_creates_schema() {
        let db = Database::open_in_memory().expect("failed to open in-memory db");

        let table_exists: bool = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='commits_cache'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("query failed")
            > 0;

        assert!(table_exists, "commits_cache table should exist");
    }

    #[test]
    fn test_open_in_memory_creates_schema_version() {
        let db = Database::open_in_memory().expect("failed to open in-memory db");

        let version: i64 = db
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("pragma failed");

        assert_eq!(version, 2, "user_version should be 2 after migration");
    }

    #[test]
    fn test_open_file_creates_db() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let db_path = dir.path().join("test.db");

        assert!(!db_path.exists(), "db file should not exist before open");

        let _db = Database::open(&db_path).expect("failed to open file db");

        assert!(db_path.exists(), "db file should exist after open");
    }
}
