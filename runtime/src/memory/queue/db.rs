use super::types::MemoryExtractionTask;
use rusqlite::{OptionalExtension, params};

pub(super) fn queue_count(conn: &rusqlite::Connection, status: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(*) FROM memory_extraction_queue WHERE status = ?1;",
        params![status],
        |row| row.get(0),
    )
    .map_err(|error| error.to_string())
}

pub(super) fn latest_failed_error(conn: &rusqlite::Connection) -> Result<Option<String>, String> {
    conn.query_row(
        r#"
        SELECT error
        FROM memory_extraction_queue
        WHERE status = 'failed' AND error IS NOT NULL AND error != ''
        ORDER BY enqueued_at DESC
        LIMIT 1;
        "#,
        [],
        |row| row.get(0),
    )
    .optional()
    .map_err(|error| error.to_string())
}

pub(super) fn memory_task_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<MemoryExtractionTask> {
    Ok(MemoryExtractionTask {
        id: row.get(0)?,
        project_id: row.get(1)?,
        tool: row.get(2)?,
        session_id: row.get(3)?,
        transcript_path: row.get(4)?,
        workspace_path: row.get(5)?,
        source_fingerprint: row.get(6)?,
        status: row.get(7)?,
        attempts: row.get(8)?,
        error: row.get(9)?,
        enqueued_at: row.get(10)?,
    })
}
