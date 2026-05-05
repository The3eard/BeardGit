//! History of executed requests with auto-prune to last N per source.

use serde::{Deserialize, Serialize};

use crate::{database::RequestsDatabase, error::RequestsStoreError};

pub const HISTORY_CAP_PER_SOURCE: i64 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub source_kind: String,
    pub source_path: String,
    pub env_name: Option<String>,
    pub request_snapshot_json: String,
    pub response_status: Option<i64>,
    pub response_headers_json: Option<String>,
    pub response_body_blob: Option<Vec<u8>>,
    pub response_truncated: bool,
    pub duration_ms: i64,
    pub executed_at: i64,
}

#[derive(Debug, Clone)]
pub struct HistoryInsert<'a> {
    pub source_kind: &'a str,
    pub source_path: &'a str,
    pub env_name: Option<&'a str>,
    pub request_snapshot_json: &'a str,
    pub response_status: Option<i64>,
    pub response_headers_json: Option<&'a str>,
    pub response_body_blob: Option<&'a [u8]>,
    pub response_truncated: bool,
    pub duration_ms: i64,
    pub executed_at: i64,
}

impl RequestsDatabase {
    pub fn insert_history(&self, entry: HistoryInsert<'_>) -> Result<i64, RequestsStoreError> {
        self.conn.execute(
            "INSERT INTO requests_history (
                source_kind, source_path, env_name, request_snapshot_json,
                response_status, response_headers_json, response_body_blob,
                response_truncated, duration_ms, executed_at
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                entry.source_kind,
                entry.source_path,
                entry.env_name,
                entry.request_snapshot_json,
                entry.response_status,
                entry.response_headers_json,
                entry.response_body_blob,
                entry.response_truncated as i64,
                entry.duration_ms,
                entry.executed_at,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.prune_history(entry.source_kind, entry.source_path)?;
        Ok(id)
    }

    fn prune_history(
        &self,
        source_kind: &str,
        source_path: &str,
    ) -> Result<(), RequestsStoreError> {
        self.conn.execute(
            "DELETE FROM requests_history
             WHERE id IN (
               SELECT id FROM requests_history
               WHERE source_kind = ?1 AND source_path = ?2
               ORDER BY executed_at DESC
               LIMIT -1 OFFSET ?3
             )",
            rusqlite::params![source_kind, source_path, HISTORY_CAP_PER_SOURCE],
        )?;
        Ok(())
    }

    pub fn list_history(
        &self,
        source_kind: &str,
        source_path: &str,
        limit: i64,
    ) -> Result<Vec<HistoryEntry>, RequestsStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_kind, source_path, env_name, request_snapshot_json,
                    response_status, response_headers_json, response_body_blob,
                    response_truncated, duration_ms, executed_at
             FROM requests_history
             WHERE source_kind = ?1 AND source_path = ?2
             ORDER BY executed_at DESC
             LIMIT ?3",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![source_kind, source_path, limit], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    source_kind: row.get(1)?,
                    source_path: row.get(2)?,
                    env_name: row.get(3)?,
                    request_snapshot_json: row.get(4)?,
                    response_status: row.get(5)?,
                    response_headers_json: row.get(6)?,
                    response_body_blob: row.get(7)?,
                    response_truncated: row.get::<_, i64>(8)? != 0,
                    duration_ms: row.get(9)?,
                    executed_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_history_by_id(&self, id: i64) -> Result<Option<HistoryEntry>, RequestsStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_kind, source_path, env_name, request_snapshot_json,
                    response_status, response_headers_json, response_body_blob,
                    response_truncated, duration_ms, executed_at
             FROM requests_history WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(HistoryEntry {
                id: row.get(0)?,
                source_kind: row.get(1)?,
                source_path: row.get(2)?,
                env_name: row.get(3)?,
                request_snapshot_json: row.get(4)?,
                response_status: row.get(5)?,
                response_headers_json: row.get(6)?,
                response_body_blob: row.get(7)?,
                response_truncated: row.get::<_, i64>(8)? != 0,
                duration_ms: row.get(9)?,
                executed_at: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ins(db: &RequestsDatabase, path: &str, ts: i64) -> i64 {
        db.insert_history(HistoryInsert {
            source_kind: "project",
            source_path: path,
            env_name: Some("dev"),
            request_snapshot_json: "{}",
            response_status: Some(200),
            response_headers_json: Some("{}"),
            response_body_blob: Some(b"ok"),
            response_truncated: false,
            duration_ms: 10,
            executed_at: ts,
        })
        .unwrap()
    }

    #[test]
    fn insert_and_list() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        ins(&db, "users/get.http", 1);
        ins(&db, "users/get.http", 2);
        let rows = db.list_history("project", "users/get.http", 10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].executed_at, 2);
    }

    #[test]
    fn auto_prune_to_50_per_source() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        for i in 0..55 {
            ins(&db, "p.http", i);
        }
        let rows = db.list_history("project", "p.http", 100).unwrap();
        assert_eq!(rows.len(), 50);
        assert_eq!(rows[0].executed_at, 54);
        assert_eq!(rows[49].executed_at, 5);
    }

    #[test]
    fn prune_is_per_source() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        for i in 0..51 {
            ins(&db, "a.http", i);
        }
        ins(&db, "b.http", 200);
        let a = db.list_history("project", "a.http", 100).unwrap();
        let b = db.list_history("project", "b.http", 100).unwrap();
        assert_eq!(a.len(), 50);
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn get_by_id() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let id = ins(&db, "x.http", 1);
        let row = db.get_history_by_id(id).unwrap().unwrap();
        assert_eq!(row.id, id);
    }
}
