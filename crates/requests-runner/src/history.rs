//! Persistence helper that records execution results into requests-store.

use requests_store::{HistoryInsert, RequestsDatabase};

use crate::{
    error::RequestsError,
    types::{ExecutionResult, ResolvedRequest},
};

pub fn record(
    db: &RequestsDatabase,
    source_kind: &str,
    source_path: &str,
    env_name: Option<&str>,
    request: &ResolvedRequest,
    result: &ExecutionResult,
    executed_at: i64,
) -> Result<i64, RequestsError> {
    let snapshot = serde_json::to_string(request)?;
    let headers = serde_json::to_string(&result.headers)?;
    let id = db.insert_history(HistoryInsert {
        source_kind,
        source_path,
        env_name,
        request_snapshot_json: &snapshot,
        response_status: Some(result.status as i64),
        response_headers_json: Some(&headers),
        response_body_blob: Some(&result.body),
        response_truncated: result.truncated,
        duration_ms: result.duration_ms as i64,
        executed_at,
    })?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HttpMethod;

    #[test]
    fn record_writes_a_row() {
        let db = RequestsDatabase::open_in_memory().unwrap();
        let req = ResolvedRequest {
            method: HttpMethod::Get,
            url: "https://x".into(),
            ..Default::default()
        };
        let res = ExecutionResult {
            status: 200,
            headers: vec![],
            body: b"ok".to_vec(),
            truncated: false,
            duration_ms: 12,
        };
        let id = record(&db, "project", "x.http", Some("dev"), &req, &res, 1).unwrap();
        let entry = db.get_history_by_id(id).unwrap().unwrap();
        assert_eq!(entry.response_status, Some(200));
    }
}
