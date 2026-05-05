//! CRUD over the global library: collections and items.

use serde::{Deserialize, Serialize};

use crate::{database::RequestsDatabase, error::RequestsStoreError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalCollection {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalItem {
    pub id: i64,
    pub collection_id: Option<i64>,
    pub name: String,
    pub http_content: String,
    pub updated_at: i64,
}

impl RequestsDatabase {
    pub fn create_global_collection(
        &self,
        name: &str,
        parent_id: Option<i64>,
        now: i64,
    ) -> Result<i64, RequestsStoreError> {
        self.conn.execute(
            "INSERT INTO requests_global_collections (name, parent_id, created_at)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![name, parent_id, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_global_collections(&self) -> Result<Vec<GlobalCollection>, RequestsStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, parent_id, created_at FROM requests_global_collections
             ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(GlobalCollection {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    parent_id: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_global_item(
        &self,
        collection_id: Option<i64>,
        name: &str,
        http_content: &str,
        now: i64,
    ) -> Result<i64, RequestsStoreError> {
        self.conn.execute(
            "INSERT INTO requests_global_items (collection_id, name, http_content, updated_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![collection_id, name, http_content, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_global_item(
        &self,
        id: i64,
        http_content: &str,
        now: i64,
    ) -> Result<(), RequestsStoreError> {
        self.conn.execute(
            "UPDATE requests_global_items SET http_content = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![http_content, now, id],
        )?;
        Ok(())
    }

    pub fn delete_global_item(&self, id: i64) -> Result<(), RequestsStoreError> {
        self.conn.execute(
            "DELETE FROM requests_global_items WHERE id = ?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    /// Rename a global item by id.
    ///
    /// Updates only the `name` column; the `http_content` and timestamp
    /// stay untouched so a rename never looks like an "edit" in history
    /// or in the updated_at-driven sort. Idempotent at the SQL level —
    /// a missing id silently affects 0 rows and returns `Ok(())`.
    ///
    /// Used by the Requests-panel context menu's Rename action for global
    /// (DB-backed) items. Project (file-on-disk) renames don't go through
    /// here; they call `std::fs::rename` instead.
    pub fn rename_global_item(&self, id: i64, name: &str) -> Result<(), RequestsStoreError> {
        self.conn.execute(
            "UPDATE requests_global_items SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, id],
        )?;
        Ok(())
    }

    pub fn list_global_items(
        &self,
        collection_id: Option<i64>,
    ) -> Result<Vec<GlobalItem>, RequestsStoreError> {
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(GlobalItem {
                id: row.get(0)?,
                collection_id: row.get(1)?,
                name: row.get(2)?,
                http_content: row.get(3)?,
                updated_at: row.get(4)?,
            })
        };
        let rows = match collection_id {
            Some(cid) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, collection_id, name, http_content, updated_at
                     FROM requests_global_items WHERE collection_id = ?1 ORDER BY name ASC",
                )?;
                stmt.query_map([cid], map_row)?
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, collection_id, name, http_content, updated_at
                     FROM requests_global_items WHERE collection_id IS NULL ORDER BY name ASC",
                )?;
                stmt.query_map([], map_row)?
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_list_collection() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let id = db
            .create_global_collection("Forge APIs", None, 1000)
            .unwrap();
        let cols = db.list_global_collections().unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, id);
        assert_eq!(cols[0].name, "Forge APIs");
    }

    #[test]
    fn create_update_delete_item() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let cid = db.create_global_collection("c", None, 1).unwrap();
        let id = db
            .create_global_item(Some(cid), "ping", "GET /ping\n", 1)
            .unwrap();
        let items = db.list_global_items(Some(cid)).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].http_content, "GET /ping\n");

        db.update_global_item(id, "GET /pong\n", 2).unwrap();
        let items = db.list_global_items(Some(cid)).unwrap();
        assert_eq!(items[0].http_content, "GET /pong\n");

        db.delete_global_item(id).unwrap();
        let items = db.list_global_items(Some(cid)).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn rename_global_item_updates_name_only() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let cid = db.create_global_collection("c", None, 1).unwrap();
        let id = db
            .create_global_item(Some(cid), "old-name", "GET /\n", 1)
            .unwrap();

        db.rename_global_item(id, "new-name").unwrap();

        let items = db.list_global_items(Some(cid)).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "new-name");
        // http_content + updated_at are intentionally untouched.
        assert_eq!(items[0].http_content, "GET /\n");
        assert_eq!(items[0].updated_at, 1);
    }

    #[test]
    fn rename_missing_id_is_a_silent_noop() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        // No rows exist; UPDATE affects 0 rows and returns Ok.
        db.rename_global_item(9999, "ghost").unwrap();
    }

    #[test]
    fn list_items_with_no_collection() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        db.create_global_item(None, "loose", "GET /\n", 1).unwrap();
        let items = db.list_global_items(None).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "loose");
    }
}
