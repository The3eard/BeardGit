//! SQLite database wrapper for the Requests panel data.

use std::path::Path;

use rusqlite::Connection;

use crate::error::RequestsStoreError;

/// Wrapper owning the `rusqlite::Connection` for `requests.db`.
pub struct RequestsDatabase {
    pub(crate) conn: Connection,
}

impl RequestsDatabase {
    /// Open or create the requests database at the given path; runs migrations.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, RequestsStoreError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory database; runs migrations. Used by tests.
    pub fn open_in_memory() -> Result<Self, RequestsStoreError> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    fn run_migrations(&self) -> Result<(), RequestsStoreError> {
        let version: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 1 {
            self.migrate_v1()?;
        }
        Ok(())
    }

    fn migrate_v1(&self) -> Result<(), RequestsStoreError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS requests_global_collections (
                id         INTEGER PRIMARY KEY,
                name       TEXT NOT NULL,
                parent_id  INTEGER REFERENCES requests_global_collections(id),
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS requests_global_items (
                id            INTEGER PRIMARY KEY,
                collection_id INTEGER REFERENCES requests_global_collections(id),
                name          TEXT NOT NULL,
                http_content  TEXT NOT NULL,
                updated_at    INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS requests_history (
                id                    INTEGER PRIMARY KEY,
                source_kind           TEXT NOT NULL,
                source_path           TEXT NOT NULL,
                env_name              TEXT,
                request_snapshot_json TEXT NOT NULL,
                response_status       INTEGER,
                response_headers_json TEXT,
                response_body_blob    BLOB,
                response_truncated    INTEGER NOT NULL DEFAULT 0,
                duration_ms           INTEGER NOT NULL,
                executed_at           INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS requests_history_source
                ON requests_history(source_kind, source_path, executed_at DESC);

            CREATE TABLE IF NOT EXISTS requests_project_state (
                project_path     TEXT PRIMARY KEY,
                active_env       TEXT,
                last_open_request TEXT,
                divider_position REAL,
                updated_at       INTEGER NOT NULL
            );

            PRAGMA user_version = 1;
            ",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_creates_all_tables() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        for tbl in [
            "requests_global_collections",
            "requests_global_items",
            "requests_history",
            "requests_project_state",
        ] {
            let count: i64 = db
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [tbl],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "table {tbl} should exist");
        }
    }

    #[test]
    fn open_in_memory_sets_version_to_1() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let v: i64 = db
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn open_file_creates_db_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("requests.db");
        assert!(!path.exists());
        let _db = RequestsDatabase::open(&path).unwrap();
        assert!(path.exists());
    }
}
