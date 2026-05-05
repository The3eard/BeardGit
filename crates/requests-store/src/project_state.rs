//! Per-project ephemeral state for the Requests panel.

use serde::{Deserialize, Serialize};

use crate::{database::RequestsDatabase, error::RequestsStoreError};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectState {
    pub project_path: String,
    pub active_env: Option<String>,
    pub last_open_request: Option<String>,
    pub divider_position: Option<f64>,
    pub updated_at: i64,
}

impl RequestsDatabase {
    pub fn upsert_project_state(&self, state: &ProjectState) -> Result<(), RequestsStoreError> {
        self.conn.execute(
            "INSERT INTO requests_project_state
                (project_path, active_env, last_open_request, divider_position, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_path) DO UPDATE SET
                active_env = excluded.active_env,
                last_open_request = excluded.last_open_request,
                divider_position = excluded.divider_position,
                updated_at = excluded.updated_at",
            rusqlite::params![
                state.project_path,
                state.active_env,
                state.last_open_request,
                state.divider_position,
                state.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_project_state(
        &self,
        project_path: &str,
    ) -> Result<Option<ProjectState>, RequestsStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT project_path, active_env, last_open_request, divider_position, updated_at
             FROM requests_project_state WHERE project_path = ?1",
        )?;
        let mut rows = stmt.query([project_path])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ProjectState {
                project_path: row.get(0)?,
                active_env: row.get(1)?,
                last_open_request: row.get(2)?,
                divider_position: row.get(3)?,
                updated_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_and_get() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let state = ProjectState {
            project_path: "/repo".into(),
            active_env: Some("dev".into()),
            last_open_request: Some("users/get.http".into()),
            divider_position: Some(0.5),
            updated_at: 100,
        };
        db.upsert_project_state(&state).unwrap();
        let got = db.get_project_state("/repo").unwrap().unwrap();
        assert_eq!(got.active_env.as_deref(), Some("dev"));
        assert_eq!(got.divider_position, Some(0.5));
    }

    #[test]
    fn upsert_overwrites() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let mut state = ProjectState {
            project_path: "/r".into(),
            active_env: Some("dev".into()),
            ..Default::default()
        };
        state.updated_at = 1;
        db.upsert_project_state(&state).unwrap();
        state.active_env = Some("prod".into());
        state.updated_at = 2;
        db.upsert_project_state(&state).unwrap();
        let got = db.get_project_state("/r").unwrap().unwrap();
        assert_eq!(got.active_env.as_deref(), Some("prod"));
    }

    #[test]
    fn get_missing_returns_none() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        assert!(db.get_project_state("/nope").unwrap().is_none());
    }
}
