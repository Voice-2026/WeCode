pub(super) fn load_recent_entries(
    conn: &Connection,
    project_id: Option<&str>,
) -> Result<Vec<MemoryEntrySummary>, String> {
    let (sql, values) = if let Some(project_id) = project_id {
        (
            r#"
            SELECT id, scope, project_id, tool_id, tier, kind, COALESCE(module_key, 'general'),
                   status, content, rationale, source_tool, source_session_id, merged_summary_id,
                   archived_at, access_count, created_at, updated_at
            FROM memory_entries
            WHERE status IN ('active', 'archived') AND (project_id = ?1 OR scope = 'user')
            ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END, updated_at DESC
            LIMIT 10
            "#,
            vec![project_id.to_string()],
        )
    } else {
        (
            r#"
            SELECT id, scope, project_id, tool_id, tier, kind, COALESCE(module_key, 'general'),
                   status, content, rationale, source_tool, source_session_id, merged_summary_id,
                   archived_at, access_count, created_at, updated_at
            FROM memory_entries
            WHERE status IN ('active', 'archived')
            ORDER BY CASE status WHEN 'active' THEN 0 ELSE 1 END, updated_at DESC
            LIMIT 10
            "#,
            Vec::new(),
        )
    };

    let mut statement = conn.prepare(sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), |row| {
            memory_entry_summary_from_row(row, true)
        })
        .map_err(|error| error.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub(super) fn list_entries_for_management(
    conn: &Connection,
    scope: &str,
    project_id: Option<&str>,
    tier: Option<&str>,
    status: Option<&str>,
    limit: i64,
) -> Result<Vec<MemoryEntrySummary>, String> {
    let mut clauses = vec![
        "scope = ?".to_string(),
        "COALESCE(project_id, '') = COALESCE(?, '')".to_string(),
    ];
    let mut values = vec![
        rusqlite::types::Value::Text(normalize_scope(scope).to_string()),
        optional_sql_text(if normalize_scope(scope) == "project" {
            project_id
        } else {
            None
        }),
    ];
    if let Some(tier) = tier {
        clauses.push("tier = ?".to_string());
        values.push(rusqlite::types::Value::Text(tier.to_string()));
    }
    if let Some(status) = status {
        if status == "archived" {
            clauses.push("status IN ('archived', 'merged')".to_string());
        } else {
            clauses.push("status = ?".to_string());
            values.push(rusqlite::types::Value::Text(status.to_string()));
        }
    }
    values.push(rusqlite::types::Value::Integer(limit));
    let sql = format!(
        r#"
        SELECT {}
        FROM memory_entries
        WHERE {}
        ORDER BY updated_at DESC, created_at DESC
        LIMIT ?
        "#,
        entry_select_columns(),
        clauses.join(" AND ")
    );
    let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), |row| {
            memory_entry_summary_from_row(row, false)
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn memory_entry_summary_from_row(
    row: &rusqlite::Row<'_>,
    truncate_content: bool,
) -> rusqlite::Result<MemoryEntrySummary> {
    let content = row.get::<_, String>(8)?;
    Ok(MemoryEntrySummary {
        id: row.get(0)?,
        scope: row.get(1)?,
        project_id: row.get(2)?,
        tool_id: row.get(3)?,
        tier: row.get(4)?,
        kind: row.get(5)?,
        module_key: row.get(6)?,
        status: row.get(7)?,
        content: if truncate_content {
            truncate(content, 96)
        } else {
            content
        },
        rationale: row.get(9)?,
        source_tool: row.get(10)?,
        source_session_id: row.get(11)?,
        merged_summary_id: row.get(12)?,
        archived_at: row.get(13)?,
        access_count: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn entry_select_columns() -> &'static str {
    "id, scope, project_id, tool_id, tier, kind, COALESCE(module_key, 'general'), status, content, rationale, source_tool, source_session_id, merged_summary_id, archived_at, access_count, created_at, updated_at"
}
